import Foundation

extension NodStore {
  public func register(pushToken: String? = nil) async {
    do {
      guard !baseURLString.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else {
        throw NodAPIError.badURL
      }

      let normalizedURL = NodServerAddress.normalizedBaseURL(baseURLString)
      guard let url = URL(string: normalizedURL) else {
        throw NodAPIError.badURL
      }

      let api = NodAPI(baseURL: url)
      let registrationPushToken = pushToken ?? self.pushToken
      let nativeAppId = try Self.nativeAppId(requiredForPushToken: registrationPushToken)
      let profileId = NodServerAddress.profileId(for: normalizedURL)
      let signingKey = try signingKeys.signingKey(account: Self.signingKeyAccount(for: profileId))
      let attestationRequest = NodAppAttestationRequest(
        code: normalizedEnrollmentCode,
        deviceName: deviceName,
        platform: platform,
        pushProvider: registrationPushToken == nil ? nil : Self.applePushProvider,
        pushToken: registrationPushToken,
        signingKey: signingKey,
        account: Self.appAttestKeyAccount(for: profileId)
      )
      // App Attest hardens enrollment when Apple can issue an attestation, but the
      // Secure Enclave decision-signing key remains the required device identity.
      let attestation = try? await appAttest.enrollmentAttestation(for: attestationRequest)
      let response = try await api.enroll(NodEnrollmentRequest(
        code: normalizedEnrollmentCode,
        deviceName: deviceName,
        platform: platform,
        nativeAppId: nativeAppId,
        pushProvider: registrationPushToken == nil ? nil : Self.applePushProvider,
        pushToken: registrationPushToken,
        signingKey: signingKey,
        attestation: attestation
      ))

      try applyEnrollment(
        response,
        profileId: profileId,
        baseURLString: normalizedURL
      )
      await syncDevicePreferences()
      connectSync()
      await refresh()
    } catch {
      lastError = error.localizedDescription
    }
  }

  public func revokeCurrentDevice() async {
    guard let server = selectedServer, let deviceId = server.deviceId else {
      return
    }
    do {
      try await api(for: server)?.revokeDevice(id: deviceId)
      removeServersLocally([server.id])
      lastError = nil
    } catch NodAPIError.badStatus(let status, _) where status == 401 || status == 403 || status == 404 {
      removeServersLocally([server.id])
      lastError = nil
    } catch {
      lastError = error.localizedDescription
    }
  }

  public func revokeDevice(_ device: NodUserDevice) async {
    guard device.isCurrent else {
      do {
        guard let api = api() else {
          throw NodAPIError.badURL
        }
        try await api.revokeDevice(id: device.id)
        registeredDevices.removeAll { $0.id == device.id }
        lastError = nil
      } catch {
        handleAuthenticatedRequestError(error)
      }
      return
    }
    await revokeCurrentDevice()
  }

  public func forgetServers(_ serverIds: [String]) {
    removeServersLocally(serverIds)
  }

  public func renameDevice(_ device: NodUserDevice, name: String) async {
    let trimmed = name.trimmingCharacters(in: .whitespacesAndNewlines)
    guard !trimmed.isEmpty else {
      return
    }
    do {
      guard let api = api() else {
        throw NodAPIError.badURL
      }
      let updated = try await api.renameDevice(id: device.id, name: trimmed)
      if let index = registeredDevices.firstIndex(where: { $0.id == updated.id }) {
        registeredDevices[index] = updated
      }
      if updated.isCurrent, let serverId = selectedServerId,
        let index = servers.firstIndex(where: { $0.id == serverId })
      {
        servers[index].deviceName = updated.name
        Self.saveServers(servers, to: defaults)
      }
      lastError = nil
    } catch {
      handleAuthenticatedRequestError(error)
    }
  }

  public func refreshAccount() async {
    do {
      guard let api = api() else {
        currentUser = nil
        registeredDevices = []
        notificationDeliveryMode = .push
        return
      }
      let me = try await api.currentUser()
      currentUser = me.user
      apply(notificationDelivery: me.notificationDelivery)
      registeredDevices = try await api.devices()
      updateSelectedServer(from: me)
      markServerContactSucceeded()
      lastError = nil
    } catch {
      handleAuthenticatedRequestError(error)
    }
  }

  public func registerPushToken(_ token: String) async {
    pushToken = token
    guard let nativeAppId = try? Self.nativeAppId(requiredForPushToken: token) else {
      lastError = NodAPIError.missingNativeAppId.localizedDescription
      return
    }
    guard !servers.isEmpty else {
      return
    }
    for server in servers {
      do {
        try await api(for: server)?.updatePushToken(
          provider: Self.applePushProvider,
          nativeAppId: nativeAppId,
          token: token
        )
        try await api(for: server)?.updateDevicePreferences(notificationSound: notificationSound)
        markServerContactSucceeded(serverId: server.id)
      } catch {
        reportConnectionError(error, serverId: server.id)
      }
    }
  }

  var normalizedEnrollmentCode: String {
    enrollmentCode.trimmingCharacters(in: .whitespacesAndNewlines).uppercased()
  }

  func removeServersLocally(_ serverIds: [String]) {
    let serverIdsToRemove = Set(serverIds)
    guard !serverIdsToRemove.isEmpty, servers.contains(where: { serverIdsToRemove.contains($0.id) })
    else {
      return
    }

    let removedSelectedServer = selectedServer.map { serverIdsToRemove.contains($0.id) } ?? false
    for serverId in serverIdsToRemove {
      try? keychain.delete(account: Self.tokenAccount(for: serverId))
      try? signingKeys.delete(account: Self.signingKeyAccount(for: serverId))
      try? appAttest.delete(account: Self.appAttestKeyAccount(for: serverId))
      tokenCache[serverId] = nil
      loadedTokenServerIds.remove(serverId)
    }
    clearServerConnectionIssues(for: serverIdsToRemove)

    servers.removeAll { serverIdsToRemove.contains($0.id) }
    Self.saveServers(servers, to: defaults)
    let selectedServerStillExists =
      selectedServerId.map { selectedServerId in
        servers.contains { $0.id == selectedServerId }
      } ?? false
    if removedSelectedServer || !selectedServerStillExists {
      selectedServerId = servers.first?.id
    }
    if let selectedServerId {
      defaults.set(selectedServerId, forKey: "nod.selectedServerId")
    } else {
      defaults.removeObject(forKey: "nod.selectedServerId")
    }
    isRegistered = !servers.isEmpty
    guard removedSelectedServer || !isRegistered else {
      return
    }

    currentUser = nil
    registeredDevices = []
    channels = []
    pendingCountsByChannel = [:]
    events = []
    notificationDeliveryMode = .push
    resetKnownPendingEvents()
    selectedChannelId = nil
    selectedEventId = nil
    sync.disconnect()
    isSyncConnected = false
    if isRegistered {
      connectSync()
      Task { await refresh() }
    }
  }

  static func nativeAppId(requiredForPushToken pushToken: String?) throws -> String? {
    let nativeAppId = Bundle.main.bundleIdentifier?
      .trimmingCharacters(in: .whitespacesAndNewlines)
    guard let nativeAppId, !nativeAppId.isEmpty else {
      if pushToken == nil {
        return nil
      }
      throw NodAPIError.missingNativeAppId
    }
    return nativeAppId
  }

  private func applyEnrollment(
    _ response: EnrollDeviceResponse,
    profileId: String,
    baseURLString: String
  ) throws {
    let profile = NodServerProfile(
      id: profileId,
      name: NodServerAddress.displayName(for: baseURLString),
      baseURLString: baseURLString,
      deviceName: deviceName,
      deviceId: response.deviceId,
      userId: response.userId,
      userName: response.userName
    )

    if let index = servers.firstIndex(where: { $0.id == profileId }) {
      servers[index] = profile
    } else {
      servers.append(profile)
    }

    try keychain.save(response.token, account: Self.tokenAccount(for: profileId))
    tokenCache[profileId] = response.token
    loadedTokenServerIds.insert(profileId)
    selectedServerId = profileId
    currentUser = NodUser(
      id: response.userId,
      name: response.userName,
      createdAt: Date(),
      updatedAt: Date()
    )
    apply(notificationDelivery: response.notificationDelivery)
    registeredDevices = response.devices
    channels = response.channels
    resetKnownPendingEvents()
    selectedChannelId = response.channels.first(where: \.subscribed)?.id ?? response.channels.first?.id
    selectedEventId = nil
    defaults.set(profileId, forKey: "nod.selectedServerId")
    defaults.set(baseURLString, forKey: "nod.baseURL")
    defaults.set(deviceName, forKey: "nod.deviceName")
    defaults.set(response.deviceId, forKey: "nod.deviceId")
    Self.saveServers(servers, to: defaults)
    enrollmentCode = ""
    isRegistered = true
  }

  private func updateSelectedServer(from response: CurrentUserResponse) {
    guard let serverId = selectedServerId,
      let index = servers.firstIndex(where: { $0.id == serverId })
    else {
      return
    }
    servers[index].userId = response.user.id
    servers[index].userName = response.user.name
    servers[index].deviceId = response.currentDevice.id
    servers[index].deviceName = response.currentDevice.name
    Self.saveServers(servers, to: defaults)
  }
}
