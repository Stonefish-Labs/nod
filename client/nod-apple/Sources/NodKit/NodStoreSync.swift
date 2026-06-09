import Darwin
import Foundation

enum NodAuthenticatedRequestErrorAction: Equatable, Sendable {
  case serverRejectedSession
  case missingLocalToken
  case requestDenied
  case connectionIssue
}

extension NodStore {
  public func connectSync() {
    syncReconnectTask?.cancel()
    syncReconnectTask = nil
    do {
      guard let api = api() else {
        return
      }
      sync.disconnect()
      isSyncConnected = false
      sync.connect(url: try api.websocketURL())
      Task { await syncDevicePreferences() }
    } catch {
      lastError = error.localizedDescription
    }
  }

  public func disconnectSync() {
    syncReconnectTask?.cancel()
    syncReconnectTask = nil
    sync.disconnect()
    isSyncConnected = false
  }

  public func resumeFromForeground() async {
    guard isRegistered else {
      return
    }
    await refresh()
    connectSync()
  }

  func handle(envelope: NodSyncEnvelope) async {
    markServerContactSucceeded()
    isSyncConnected = true
    if let notificationDelivery = envelope.notificationDelivery {
      apply(notificationDelivery: notificationDelivery)
    }
    if let source = envelope.source {
      if let index = sources.firstIndex(where: { $0.id == source.id }) {
        sources[index] = source
      } else {
        sources.append(source)
      }
    }
    if let request = envelope.request {
      await applySyncedRequest(request, envelopeKind: envelope.kind)
    }
    if envelope.kind == "cleared" || envelope.kind == "subscription_updated" {
      await refresh()
    }
    if envelope.kind.hasPrefix("device_") {
      await refreshAccount()
    }
  }

  func handleSyncError(_ error: Error) {
    isSyncConnected = false
    if Self.isExpectedSyncDisconnect(error) {
      reportConnectionError(error)
      scheduleSyncReconnect()
      return
    }
    if Self.isTransientConnectionError(error) {
      reportConnectionError(error)
      scheduleSyncReconnect()
      return
    }
    reportConnectionError(error)
    scheduleSyncReconnect()
  }

  func reportConnectionError(_ error: Error, serverId: String? = nil) {
    markServerConnectionIssue(error.localizedDescription, serverId: serverId ?? selectedServer?.id)
  }

  func handleAuthenticatedRequestError(_ error: Error) {
    switch Self.authenticatedRequestErrorAction(for: error) {
    case .serverRejectedSession:
      reportSelectedServerSessionInvalid()
    case .missingLocalToken:
      reportSelectedServerTokenMissing()
    case .requestDenied:
      reportDeniedAuthenticatedRequest(error)
    case .connectionIssue:
      reportConnectionError(error)
    }
  }

  nonisolated static func authenticatedRequestErrorAction(
    for error: Error
  ) -> NodAuthenticatedRequestErrorAction {
    if case NodAPIError.badStatus(let status, _) = error {
      if status == 401 {
        return .serverRejectedSession
      }
      if status == 403 {
        return .requestDenied
      }
      return .connectionIssue
    }
    if case NodAPIError.missingToken = error {
      return .missingLocalToken
    }
    return .connectionIssue
  }

  private func reportSelectedServerSessionInvalid() {
    let message: String
    if let server = selectedServer {
      message =
        "Your Nod session with \(server.name) is no longer valid. Re-enroll this device to continue."
      reEnrollmentServerId = server.id
      tokenCache[server.id] = nil
      loadedTokenServerIds.remove(server.id)
      markServerConnectionIssue(message, serverId: server.id)
    } else {
      message = "Your Nod session is no longer valid. Re-enroll this device to continue."
      reEnrollmentServerId = nil
    }
    disconnectSync()
    lastError = message
  }

  private func reportSelectedServerTokenMissing() {
    let message: String
    if let server = selectedServer {
      message =
        "The saved Nod token for \(server.name) is missing. Re-enroll this device or forget this server."
      reEnrollmentServerId = server.id
      markServerConnectionIssue(message, serverId: server.id)
    } else {
      message = "The saved Nod token is missing. Re-enroll this device to continue."
      reEnrollmentServerId = nil
    }
    disconnectSync()
    lastError = message
  }

  private func reportDeniedAuthenticatedRequest(_ error: Error) {
    if let serverId = selectedServer?.id {
      markServerConnectionIssue(error.localizedDescription, serverId: serverId)
    }
    lastError = error.localizedDescription
  }

  func markServerContactSucceeded(serverId: String? = nil) {
    guard let serverId = serverId ?? selectedServer?.id else {
      return
    }
    clearServerConnectionIssues(for: [serverId])
  }

  private func applySyncedRequest(_ request: NodRequest, envelopeKind: String) async {
    if request.status == .pending {
      pendingCountsBySource[request.sourceId, default: 0] += envelopeKind == "created" ? 1 : 0
      knownPendingRequestIds.insert(request.id)
    } else {
      pendingCountsBySource[request.sourceId] = max(
        0,
        (pendingCountsBySource[request.sourceId] ?? 1) - 1
      )
      knownPendingRequestIds.remove(request.id)
    }
    if selectedSourceId == nil || selectedSourceId == request.sourceId {
      upsert(request)
    }
    if envelopeKind == "created", shouldPresentLocalNotificationFromSync() {
      await presentLocalNotification(for: request)
    }
  }

  private func markServerConnectionIssue(_ message: String, serverId: String?) {
    guard let serverId else {
      return
    }
    var issues = serverConnectionIssuesById
    issues[serverId] = message
    serverConnectionIssuesById = issues
  }

  func clearServerConnectionIssues<S: Sequence>(for serverIds: S)
  where S.Element == String {
    let serverIdsToClear = Set(serverIds)
    guard serverConnectionIssuesById.keys.contains(where: { serverIdsToClear.contains($0) }) else {
      return
    }
    var issues = serverConnectionIssuesById
    for serverId in serverIdsToClear {
      issues[serverId] = nil
    }
    serverConnectionIssuesById = issues
  }

  private func scheduleSyncReconnect() {
    guard isRegistered else {
      return
    }
    syncReconnectTask?.cancel()
    syncReconnectTask = Task { [weak self] in
      // A short delay avoids a tight reconnect loop while still recovering quickly
      // from Wi-Fi changes, sleep/wake, and server restarts.
      try? await Task.sleep(nanoseconds: 2_000_000_000)
      guard !Task.isCancelled else {
        return
      }
      await MainActor.run {
        self?.syncReconnectTask = nil
        self?.connectSync()
      }
    }
  }

  private static func isExpectedSyncDisconnect(_ error: Error) -> Bool {
    let nsError = error as NSError
    // Normal WebSocket closes often arrive as low-level URL or POSIX failures;
    // treat those as reconnectable connection state instead of modal errors.
    if nsError.domain == NSURLErrorDomain {
      let code = URLError.Code(rawValue: nsError.code)
      if code == .cancelled || code == .networkConnectionLost {
        return true
      }
    }
    if nsError.domain == NSPOSIXErrorDomain {
      let expectedCodes = [
        Int(ECONNABORTED),
        Int(ECONNRESET),
        Int(ENOTCONN),
        Int(EPIPE),
      ]
      if expectedCodes.contains(nsError.code) {
        return true
      }
    }
    if let underlying = nsError.userInfo[NSUnderlyingErrorKey] as? Error {
      return isExpectedSyncDisconnect(underlying)
    }
    return nsError.localizedDescription
      .localizedCaseInsensitiveContains("software caused connection abort")
  }

  private static func isTransientConnectionError(_ error: Error) -> Bool {
    if case NodAPIError.badStatus(let status, _) = error {
      return status == -1 || (500..<600).contains(status)
    }

    let nsError = error as NSError
    if nsError.domain == NSURLErrorDomain {
      let code = URLError.Code(rawValue: nsError.code)
      let transientCodes: Set<URLError.Code> = [
        .cancelled,
        .timedOut,
        .badServerResponse,
        .cannotFindHost,
        .cannotConnectToHost,
        .networkConnectionLost,
        .dnsLookupFailed,
        .notConnectedToInternet,
        .resourceUnavailable,
      ]
      if transientCodes.contains(code) {
        return true
      }
    }

    if nsError.domain == NSPOSIXErrorDomain {
      let transientCodes = [
        Int(ECONNABORTED),
        Int(ECONNRESET),
        Int(ECONNREFUSED),
        Int(ENOTCONN),
        Int(ENETDOWN),
        Int(ENETUNREACH),
        Int(EHOSTDOWN),
        Int(EHOSTUNREACH),
        Int(ETIMEDOUT),
        Int(EPIPE),
      ]
      if transientCodes.contains(nsError.code) {
        return true
      }
    }

    if nsError.domain.localizedCaseInsensitiveContains("CFNetwork") {
      return true
    }

    if let underlying = nsError.userInfo[NSUnderlyingErrorKey] as? Error {
      return isTransientConnectionError(underlying)
    }

    let description = nsError.localizedDescription
    return description.localizedCaseInsensitiveContains("timed out")
      || description.localizedCaseInsensitiveContains("could not connect")
      || description.localizedCaseInsensitiveContains("cannot find host")
      || description.localizedCaseInsensitiveContains("network connection was lost")
      || description.localizedCaseInsensitiveContains("not connected to the internet")
      || description.localizedCaseInsensitiveContains("software caused connection abort")
  }
}
