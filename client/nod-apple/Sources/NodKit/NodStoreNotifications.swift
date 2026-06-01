import Foundation

extension NodStore {
  public func requestNotifications(reportMissingGrant: Bool = true) async {
    if let issue = notificationRuntimeIssue() {
      notificationPermissionIssue = issue
      return
    }

    do {
      let authorized = try await NodNotificationController.shared.requestAuthorization()
      await updateNotificationPermissionIssue(
        authorized: authorized,
        reportMissingGrant: reportMissingGrant
      )
    } catch {
      lastError = error.localizedDescription
    }
  }

  public func requestAndTestNotifications() async {
    await requestNotifications()
    guard shouldPresentLocalNotificationFromSync(), notificationPermissionIssue == nil else {
      return
    }

    do {
      try await NodNotificationController.shared.presentTestNotification(
        soundName: notificationSound
      )
    } catch {
      let settings = await NodNotificationController.shared.notificationSettings()
      notificationPermissionIssue =
        notificationPermissionIssue(for: settings, reportMissingGrant: true)
        ?? "Could not show Nod test notification: \(error.localizedDescription)"
    }
  }

  public func openNotification(eventId: String?, channelId: String?) async {
    guard isRegistered else {
      return
    }
    if let channelId, !channelId.isEmpty {
      selectedChannelId = channelId
    }
    await refresh()
    if let channelId, !channelId.isEmpty {
      selectedChannelId = channelId
    }
    if let eventId, !eventId.isEmpty {
      selectedEventId = eventId
    }
    notificationOpenRequest = NodNotificationOpenRequest(
      eventId: selectedEventId,
      channelId: selectedChannelId ?? channelId
    )
    connectSync()
  }

  func presentNotificationsForPendingEventsDiscoveredByRefresh(_ pendingEvents: [NodEvent]) async {
    let pendingEventIds = Set(pendingEvents.map(\.id))
    defer {
      knownPendingEventIds = pendingEventIds
      hasLoadedPendingEventSnapshot = true
    }

    // The first refresh seeds the snapshot so an existing backlog does not replay
    // as a burst of local notifications after launch or server switching.
    guard hasLoadedPendingEventSnapshot, shouldPresentLocalNotificationFromSync() else {
      return
    }

    for event in pendingEvents where !knownPendingEventIds.contains(event.id) {
      await presentLocalNotification(for: event)
    }
  }

  func presentLocalNotification(for event: NodEvent) async {
    do {
      try await NodNotificationController.shared.presentLocalNotification(
        for: event,
        soundName: notificationSound
      )
    } catch {
      let settings = await NodNotificationController.shared.notificationSettings()
      notificationPermissionIssue =
        notificationPermissionIssue(for: settings, reportMissingGrant: true)
        ?? "Could not show Nod notification: \(error.localizedDescription)"
    }
  }

  func apply(notificationDelivery: NodNotificationDelivery) {
    notificationDeliveryMode = notificationDelivery.mode
  }

  func shouldPresentLocalNotificationFromSync() -> Bool {
    NodNotificationPolicy.shouldPresentLocalNotification(
      presentLocalNotifications: presentLocalNotifications,
      deliveryMode: notificationDeliveryMode
    )
  }

  private func updateNotificationPermissionIssue(authorized: Bool, reportMissingGrant: Bool) async {
    let settings = await NodNotificationController.shared.notificationSettings()
    notificationPermissionIssue = notificationPermissionIssue(
      for: settings,
      reportMissingGrant: reportMissingGrant && !authorized
    )
  }

  private func notificationRuntimeIssue() -> String? {
    guard shouldPresentLocalNotificationFromSync() else {
      return nil
    }

    #if os(macOS)
      if Bundle.main.bundleURL.pathExtension != "app" {
        return
          "Nod is running outside the Nod.app bundle, so macOS cannot add it to Notification Settings. Launch the built Nod.app instead of the SwiftPM executable."
      }
    #endif

    return nil
  }

  private func notificationPermissionIssue(
    for settings: NodNotificationSettings,
    reportMissingGrant: Bool
  ) -> String? {
    guard shouldPresentLocalNotificationFromSync() else {
      return nil
    }

    switch settings.authorizationStatus {
    case .denied:
      return
        "Notifications are disabled for Nod. Enable them in System Settings > Notifications to see desktop alerts."
    case .notDetermined:
      if reportMissingGrant {
        return "Nod has not been granted notification permission yet."
      }
    case .authorized, .provisional, .ephemeral, .unknown:
      break
    }

    if settings.alertSetting == .disabled {
      return
        "Nod notifications are allowed, but alert banners are disabled. Enable banners for Nod in System Settings > Notifications."
    }

    return nil
  }
}
