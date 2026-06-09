import Foundation

extension NodStore {
  public func refresh() async {
    do {
      guard let api = api() else {
        currentUser = nil
        registeredDevices = []
        channels = []
        requests = []
        notificationDeliveryMode = .push
        return
      }
      await refreshAccount()
      guard selectedServer != nil else {
        return
      }
      channels = try await api.channels()
      selectFirstVisibleChannelIfNeeded()

      let allVisibleRequests = locallyVisibleRequests(
        try await api.requests(NodRequestQuery(limit: 500))
      )
      let pendingRequests = allVisibleRequests.filter { $0.status == .pending }
      pendingCountsByChannel = NodRequestInbox.pendingCountsByChannel(in: pendingRequests)
      await presentNotificationsForPendingRequestsDiscoveredByRefresh(pendingRequests)
      if let selectedChannelId {
        requests = NodRequestInbox.visibleRequests(
          allVisibleRequests.filter { $0.channelId == selectedChannelId }
        )
      } else {
        requests = []
      }
      if let selectedRequestId, !requests.contains(where: { $0.id == selectedRequestId }) {
        self.selectedRequestId = nil
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
      applySubmittedRequest(updated, replacing: request)
      markServerContactSucceeded()
      lastError = nil
    } catch {
      await handleDecisionSubmitError(error)
    }
  }

  public func dismissIfInformational(request: NodRequest) async {
    guard request.status == .pending, request.options.isEmpty else {
      return
    }
    guard !informationalDismissSubmissions.contains(request.id) else {
      return
    }
    informationalDismissSubmissions.insert(request.id)
    dismissInformationalLocally(request)
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
      applySubmittedRequest(updated, replacing: request)
      markServerContactSucceeded()
      lastError = nil
    } catch {
      // Opening an informational request is a read receipt. Keep the local UI cleared
      // even if the best-effort server acknowledgement cannot be sent immediately.
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
      applySubmittedRequest(updated, replacing: request)
      markServerContactSucceeded()
      lastError = nil
    } catch {
      await handleDecisionSubmitError(error)
    }
  }

  public func clearSelectedChannel() async {
    guard let selectedChannelId else {
      return
    }
    do {
      guard let api = api() else {
        throw NodAPIError.badURL
      }
      try await api.clear(channelId: selectedChannelId)
      requests.removeAll { $0.channelId == selectedChannelId }
      pendingCountsByChannel[selectedChannelId] = 0
      selectedRequestId = requests.first?.id
      lastError = nil
    } catch {
      handleAuthenticatedRequestError(error)
    }
  }

  public func setSubscription(channelId: String, subscribed: Bool) async {
    do {
      guard let api = api() else {
        throw NodAPIError.badURL
      }
      try await api.updateSubscription(channelId: channelId, subscribed: subscribed)
      if let index = channels.firstIndex(where: { $0.id == channelId }) {
        channels[index].subscribed = subscribed
      }
      if !subscribed, selectedChannelId == channelId {
        selectedChannelId = subscribedChannels.first?.id
        requests = []
        pendingCountsByChannel[channelId] = nil
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
  }

  func applySubmittedRequest(_ request: NodRequest, replacing previousRequest: NodRequest) {
    reconcilePendingCounts(updated: request, replacing: previousRequest)
    upsert(request)
  }

  private func reconcilePendingCounts(updated request: NodRequest, replacing previousRequest: NodRequest) {
    guard previousRequest.status == .pending, request.status != .pending else {
      return
    }
    pendingCountsByChannel[previousRequest.channelId] = max(
      0,
      (pendingCountsByChannel[previousRequest.channelId] ?? 1) - 1
    )
    knownPendingRequestIds.remove(previousRequest.id)
  }

  private func dismissInformationalLocally(_ request: NodRequest) {
    locallyDismissedInformationalRequestIds.insert(request.id)
    saveLocallyDismissedInformationalRequestIds()
    applySubmittedRequest(localDismissedInformationalRequest(from: request), replacing: request)
    knownPendingRequestIds.remove(request.id)
  }

  private func locallyVisibleRequests(_ requests: [NodRequest]) -> [NodRequest] {
    requests.filter { request in
      !isLocallyDismissedInformational(request)
    }
  }

  private func isLocallyDismissedInformational(_ request: NodRequest) -> Bool {
    request.status == .pending
      && request.options.isEmpty
      && locallyDismissedInformationalRequestIds.contains(request.id)
  }

  private func decrementPendingCount(for channelId: String) {
    let count = max(0, (pendingCountsByChannel[channelId] ?? 1) - 1)
    pendingCountsByChannel[channelId] = count == 0 ? nil : count
  }

  private func localDismissedInformationalRequest(from request: NodRequest) -> NodRequest {
    let resolvedAt = Date()
    let decision = NodDecision(
      requestId: request.id,
      optionId: "dismiss",
      optionKind: .dismiss,
      optionLabel: "Dismiss",
      resolvedAt: resolvedAt
    )
    return NodRequest(
      id: request.id,
      requestId: request.requestId,
      channelId: request.channelId,
      recipients: request.recipients,
      decisionResolution: request.decisionResolution,
      title: request.title,
      summary: request.summary,
      bodyMarkdown: request.bodyMarkdown,
      fields: request.fields,
      links: request.links,
      imageUrl: request.imageUrl,
      notification: request.notification,
      dedupeKey: request.dedupeKey,
      expiresAt: request.expiresAt,
      status: .resolved,
      createdAt: request.createdAt,
      updatedAt: resolvedAt,
      resolvedAt: resolvedAt,
      decision: decision,
      decisions: request.decisions,
      callbackUrl: request.callbackUrl,
      options: request.options,
      requestDigest: request.requestDigest
    )
  }

  private func saveLocallyDismissedInformationalRequestIds() {
    defaults.set(
      Array(locallyDismissedInformationalRequestIds).sorted(),
      forKey: "nod.dismissedInformationalRequestIds"
    )
  }

  private func handleDecisionSubmitError(_ error: Error) async {
    if case NodAPIError.badStatus(let status, _) = error {
      if status == 401 {
        handleAuthenticatedRequestError(error)
        return
      }
      lastError = error.localizedDescription
      if status == 409 || status == 404 {
        await refresh()
      }
      return
    }

    handleAuthenticatedRequestError(error)
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

  private func selectFirstVisibleChannelIfNeeded() {
    let visibleChannels = subscribedChannels
    if selectedChannelId == nil
      || !visibleChannels.contains(where: { $0.id == selectedChannelId })
    {
      selectedChannelId = visibleChannels.first?.id
    }
  }
}
