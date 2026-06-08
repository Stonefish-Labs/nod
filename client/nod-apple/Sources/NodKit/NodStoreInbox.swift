import Foundation

extension NodStore {
  public func refresh() async {
    do {
      guard let api = api() else {
        currentUser = nil
        registeredDevices = []
        sources = []
        requests = []
        notificationDeliveryMode = .push
        return
      }
      await refreshAccount()
      guard selectedServer != nil else {
        return
      }
      sources = try await api.sources()
      selectFirstVisibleSourceIfNeeded()

      let allVisibleRequests = try await api.requests(NodRequestQuery(limit: 500))
      let pendingRequests = allVisibleRequests.filter { $0.status == .pending }
      pendingCountsBySource = NodRequestInbox.pendingCountsBySource(in: pendingRequests)
      await presentNotificationsForPendingRequestsDiscoveredByRefresh(pendingRequests)
      if let selectedSourceId {
        requests = NodRequestInbox.visibleRequests(
          allVisibleRequests.filter { $0.sourceId == selectedSourceId }
        )
      } else {
        requests = []
      }
      if selectedRequestId == nil || !requests.contains(where: { $0.id == selectedRequestId }) {
        selectedRequestId = requests.first?.id
      }
      markServerContactSucceeded()
      lastError = nil
    } catch {
      handleAuthenticatedRequestError(error)
    }
  }

  public func submit(request: NodRequest, option: NodRequestOption, text: String? = nil) async {
    do {
      guard let api = api() else {
        throw NodAPIError.badURL
      }
      let signature = try decisionSignature(for: request, option: option, text: text)
      let updated = try await api.submit(
        requestId: request.id,
        optionId: option.id,
        text: text,
        signature: signature
      )
      upsert(updated)
      lastError = nil
    } catch {
      handleAuthenticatedRequestError(error)
    }
  }

  public func dismissIfInformational(request: NodRequest) async {
    guard request.status == .pending, request.options.isEmpty else {
      return
    }
    do {
      guard let api = api() else {
        throw NodAPIError.badURL
      }
      let option = NodRequestOption(id: "dismiss", label: "Dismiss", kind: .dismiss)
      let signature = try decisionSignature(for: request, option: option)
      let updated = try await api.submit(
        requestId: request.id,
        optionId: "dismiss",
        signature: signature
      )
      upsert(updated)
      lastError = nil
    } catch {
      handleAuthenticatedRequestError(error)
    }
  }

  func submitNotificationOption(requestId: String, optionId: String, text: String?) async {
    do {
      guard let api = api() else {
        throw NodAPIError.badURL
      }
      let request = try await api.request(id: requestId)
      let option =
        request.options.first(where: { $0.id == optionId })
        ?? NodRequestOption(id: optionId, label: optionId, kind: .custom)
      let signature = try decisionSignature(for: request, option: option, text: text)
      let updated = try await api.submit(
        requestId: request.id,
        optionId: option.id,
        text: text,
        signature: signature
      )
      upsert(updated)
      lastError = nil
    } catch {
      handleAuthenticatedRequestError(error)
    }
  }

  public func clearSelectedSource() async {
    guard let selectedSourceId else {
      return
    }
    do {
      guard let api = api() else {
        throw NodAPIError.badURL
      }
      try await api.clear(sourceId: selectedSourceId)
      requests.removeAll { $0.sourceId == selectedSourceId }
      pendingCountsBySource[selectedSourceId] = 0
      selectedRequestId = requests.first?.id
      lastError = nil
    } catch {
      handleAuthenticatedRequestError(error)
    }
  }

  public func setSubscription(sourceId: String, subscribed: Bool) async {
    do {
      guard let api = api() else {
        throw NodAPIError.badURL
      }
      try await api.updateSubscription(sourceId: sourceId, subscribed: subscribed)
      if let index = sources.firstIndex(where: { $0.id == sourceId }) {
        sources[index].subscribed = subscribed
      }
      if !subscribed, selectedSourceId == sourceId {
        selectedSourceId = subscribedSources.first?.id
        requests = []
        pendingCountsBySource[sourceId] = nil
        await refresh()
      }
      lastError = nil
    } catch {
      handleAuthenticatedRequestError(error)
    }
  }

  public func setNotificationSound(_ sound: String) async {
    notificationSound = sound
    defaults.set(sound, forKey: "nod.notificationSound")
    await syncDevicePreferences()
  }

  func resetKnownPendingRequests() {
    knownPendingRequestIds = []
    hasLoadedPendingRequestSnapshot = false
  }

  func upsert(_ request: NodRequest) {
    if let index = requests.firstIndex(where: { $0.id == request.id }) {
      requests[index] = request
    } else {
      requests.insert(request, at: 0)
    }
    requests = NodRequestInbox.visibleRequests(requests)
    if selectedRequestId == nil {
      selectedRequestId = request.id
    }
  }

  private func decisionSignature(
    for request: NodRequest,
    option: NodRequestOption,
    text: String? = nil
  ) throws -> NodDecisionSignature {
    guard let server = selectedServer else {
      throw NodSigningError.missingDeviceIdentity
    }
    return try signingKeys.sign(NodDecisionSigningRequest(
      request: request,
      option: option,
      text: text,
      userId: server.userId,
      deviceId: server.deviceId,
      account: Self.signingKeyAccount(for: server.id)
    ))
  }

  private func selectFirstVisibleSourceIfNeeded() {
    let visibleSources = subscribedSources
    if selectedSourceId == nil
      || !visibleSources.contains(where: { $0.id == selectedSourceId })
    {
      selectedSourceId = visibleSources.first?.id
    }
  }
}
