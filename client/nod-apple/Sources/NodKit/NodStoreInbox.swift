import Foundation

extension NodStore {
  public func refresh() async {
    do {
      guard let api = api() else {
        currentUser = nil
        registeredDevices = []
        channels = []
        events = []
        notificationDeliveryMode = .push
        return
      }
      await refreshAccount()
      guard selectedServer != nil else {
        return
      }
      channels = try await api.channels()
      selectFirstVisibleChannelIfNeeded()

      let allVisibleEvents = try await api.events(NodEventQuery(limit: 500))
      let pendingEvents = allVisibleEvents.filter { $0.status == .pending }
      pendingCountsByChannel = NodEventInbox.pendingCountsByChannel(in: pendingEvents)
      await presentNotificationsForPendingEventsDiscoveredByRefresh(pendingEvents)
      if let selectedChannelId {
        events = NodEventInbox.visibleEvents(
          allVisibleEvents.filter { $0.channelId == selectedChannelId }
        )
      } else {
        events = []
      }
      if selectedEventId == nil || !events.contains(where: { $0.id == selectedEventId }) {
        selectedEventId = events.first?.id
      }
      markServerContactSucceeded()
      lastError = nil
    } catch {
      handleAuthenticatedRequestError(error)
    }
  }

  public func submit(event: NodEvent, action: NodAction, text: String? = nil) async {
    do {
      guard let api = api() else {
        throw NodAPIError.badURL
      }
      let signature = try decisionSignature(for: event, action: action, text: text)
      let updated = try await api.submit(
        eventId: event.id,
        actionId: action.id,
        text: text,
        signature: signature
      )
      upsert(updated)
      lastError = nil
    } catch {
      handleAuthenticatedRequestError(error)
    }
  }

  public func dismissIfInformational(event: NodEvent) async {
    guard event.status == .pending, event.actions.isEmpty else {
      return
    }
    do {
      guard let api = api() else {
        throw NodAPIError.badURL
      }
      let action = NodAction(id: "dismiss", label: "Dismiss", kind: .dismiss)
      let signature = try decisionSignature(for: event, action: action)
      let updated = try await api.submit(
        eventId: event.id,
        actionId: "dismiss",
        signature: signature
      )
      upsert(updated)
      lastError = nil
    } catch {
      handleAuthenticatedRequestError(error)
    }
  }

  func submitNotificationAction(eventId: String, actionId: String, text: String?) async {
    do {
      guard let api = api() else {
        throw NodAPIError.badURL
      }
      let event = try await api.event(id: eventId)
      let action =
        event.actions.first(where: { $0.id == actionId })
        ?? NodAction(id: actionId, label: actionId, kind: .custom)
      let signature = try decisionSignature(for: event, action: action, text: text)
      let updated = try await api.submit(
        eventId: event.id,
        actionId: action.id,
        text: text,
        signature: signature
      )
      upsert(updated)
      lastError = nil
    } catch {
      handleAuthenticatedRequestError(error)
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
      events.removeAll { $0.channelId == selectedChannelId }
      pendingCountsByChannel[selectedChannelId] = 0
      selectedEventId = events.first?.id
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
        events = []
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

  func resetKnownPendingEvents() {
    knownPendingEventIds = []
    hasLoadedPendingEventSnapshot = false
  }

  func upsert(_ event: NodEvent) {
    if let index = events.firstIndex(where: { $0.id == event.id }) {
      events[index] = event
    } else {
      events.insert(event, at: 0)
    }
    events = NodEventInbox.visibleEvents(events)
    if selectedEventId == nil {
      selectedEventId = event.id
    }
  }

  private func decisionSignature(
    for event: NodEvent,
    action: NodAction,
    text: String? = nil
  ) throws -> NodDecisionSignature {
    guard let server = selectedServer else {
      throw NodSigningError.missingDeviceIdentity
    }
    return try signingKeys.sign(NodDecisionSigningRequest(
      event: event,
      action: action,
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
