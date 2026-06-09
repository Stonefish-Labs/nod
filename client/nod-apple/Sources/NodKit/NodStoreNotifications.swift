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

  public func refreshNotificationAuthorizationStatus() async {
    let settings = await NodNotificationController.shared.notificationSettings()
    notificationAuthorizationStatus = settings.authorizationStatus
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

  public func openNotification(requestId: String?, channelId: String?) async {
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
    if let requestId, !requestId.isEmpty {
      selectedRequestId = requestId
    }
    notificationOpenRequest = NodNotificationOpenRequest(
      requestId: selectedRequestId,
      channelId: selectedChannelId ?? channelId
    )
    connectSync()
  }

  /// Present local notifications for the candidates the runtime emitted. The
  /// runtime already de-dups against its known-pending snapshot and respects
  /// the delivery mode; this just renders them through `UserNotifications`.
  func presentNotificationCandidates(_ candidates: [NodRequest]) async {
    guard shouldPresentLocalNotificationFromSync() else {
      // Drop them silently; record so a later mode flip doesn't replay a backlog.
      for request in candidates {
        presentedNotificationRequestIds.insert(request.id)
      }
      return
    }

    for request in candidates where !presentedNotificationRequestIds.contains(request.id) {
      presentedNotificationRequestIds.insert(request.id)
      await presentLocalNotification(for: request)
    }
  }

  func presentLocalNotification(for request: NodRequest) async {
    do {
      try await NodNotificationController.shared.presentLocalNotification(
        for: request,
        soundName: notificationSound
      )
    } catch {
      let settings = await NodNotificationController.shared.notificationSettings()
      notificationPermissionIssue =
        notificationPermissionIssue(for: settings, reportMissingGrant: true)
        ?? "Could not show Nod notification: \(error.localizedDescription)"
    }
  }

  func shouldPresentLocalNotificationFromSync() -> Bool {
    NodNotificationPolicy.shouldPresentLocalNotification(
      presentLocalNotifications: presentLocalNotifications,
      deliveryMode: notificationDeliveryMode
    )
  }

  private func updateNotificationPermissionIssue(authorized: Bool, reportMissingGrant: Bool) async {
    let settings = await NodNotificationController.shared.notificationSettings()
    notificationAuthorizationStatus = settings.authorizationStatus
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
