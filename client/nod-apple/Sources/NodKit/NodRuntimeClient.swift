import Foundation
import NodClientFFI

// The generated UniFFI `NodClient` wraps a thread-safe Rust object (`Arc<Mutex<…>>`),
// so it is safe to send across actors even though UniFFI 0.28's Swift bindings
// don't mark it `Sendable`.
extension NodClient: @unchecked Sendable {}

public enum NodRuntimeError: Error, LocalizedError {
  case notStarted
  case rpc(String)

  public var errorDescription: String? {
    switch self {
    case .notStarted: return "The Nod client runtime has not been started."
    case .rpc(let message): return message
    }
  }
}

/// The Apple-side driver for the shared `nod-client-core` runtime.
///
/// Owns a `NodClient` (the async Rust runtime over UniFFI), forwards its
/// `NodClientMessage` events onto the main actor as `@Published` state, and
/// sends RPCs in. This is what lets NodKit stop re-implementing the API client,
/// sync, store, and state machine — all of that now lives once in Rust (shared
/// with the TUI + desktop). The only Swift left is the native adapters: the
/// Secure Enclave signer (passed in), App Attest, UserNotifications, SwiftUI.
@MainActor
public final class NodRuntimeClient: ObservableObject {
  @Published public private(set) var state: NodRuntimeState?
  @Published public private(set) var statePath: String?
  @Published public private(set) var lastTransientError: String?
  @Published public private(set) var authRevoked: Bool = false

  /// Pending requests surfaced as local-notification candidates since the last
  /// time the host drained them.
  @Published public private(set) var notificationCandidates: [NodRequest] = []
  @Published public private(set) var removedNotificationRequestIds: [String] = []

  private var client: NodClient?
  private let signer: any NodDeviceSigner & Sendable

  public init(signer: any NodDeviceSigner & Sendable = SecureEnclaveDeviceSigner()) {
    self.signer = signer
  }

  /// Build the runtime, attach the observer, and emit the initial state.
  public func start() async throws {
    guard client == nil else { return }
    let bridge = ObserverBridge { [weak self] message in
      Task { @MainActor in self?.apply(message) }
    }
    let client = try await NodClient(observer: bridge, signer: signer)
    self.client = client
    await client.start()
  }

  // MARK: - RPCs (mirror nod-client-core's runtime methods)

  public func refresh() async throws { applyStateResult(try await call("refresh")) }

  public func selectServer(_ serverId: String) async throws {
    applyStateResult(try await call("select_server", ["server_id": serverId]))
  }

  public func forgetServer(_ serverId: String) async throws {
    applyStateResult(try await call("forget_server", ["server_id": serverId]))
  }

  public func enroll(
    baseURL: String,
    deviceName: String,
    code: String,
    notificationSound: String?,
    platform: NodDevicePlatform,
    nativeAppId: String? = nil,
    pushProvider: String? = nil,
    pushToken: String? = nil,
    attestation: [String: Any]? = nil
  ) async throws {
    var params: [String: Any] = [
      "base_url": baseURL,
      "device_name": deviceName,
      "code": code,
      "platform": platform.rawValue,
    ]
    if let notificationSound { params["notification_sound"] = notificationSound }
    // Native enrollment hardening — forwarded to the server by the runtime
    // unchanged (the SE signing key is provisioned by the signer callback).
    if let nativeAppId { params["native_app_id"] = nativeAppId }
    if let pushProvider { params["push_provider"] = pushProvider }
    if let pushToken { params["push_token"] = pushToken }
    if let attestation { params["attestation"] = attestation }
    applyStateResult(try await call("enroll", params))
  }

  public func submitOption(requestId: String, optionId: String, text: String?) async throws {
    var params: [String: Any] = ["request_id": requestId, "option_id": optionId]
    if let text { params["text"] = text }
    try await call("submit_option", params)
  }

  public func setSubscription(channelId: String, subscribed: Bool) async throws {
    applyStateResult(
      try await call("set_subscription", ["channel_id": channelId, "subscribed": subscribed]))
  }

  public func setNotificationPreference(sound: String) async throws {
    applyStateResult(try await call("set_notification_preference", ["notification_sound": sound]))
  }

  /// Register/refresh the APNs push token across every enrolled server.
  public func registerPushToken(provider: String, nativeAppId: String, token: String) async throws {
    applyStateResult(
      try await call(
        "register_push_token",
        ["provider": provider, "native_app_id": nativeAppId, "token": token]))
  }

  public func clearChannel(_ channelId: String) async throws {
    applyStateResult(try await call("clear_channel", ["channel_id": channelId]))
  }

  public func selectChannel(_ channelId: String) async throws {
    applyStateResult(try await call("select_channel", ["channel_id": channelId]))
  }

  public func selectRequest(_ requestId: String) async throws {
    applyStateResult(try await call("select_request", ["request_id": requestId]))
  }

  public func renameDevice(deviceId: String, name: String) async throws {
    try await call("rename_device", ["device_id": deviceId, "name": name])
  }

  public func revokeDevice(_ deviceId: String) async throws {
    applyStateResult(try await call("revoke_device", ["device_id": deviceId]))
  }

  public func connectSync() async throws { try await call("connect_sync") }
  public func disconnectSync() async throws { try await call("disconnect_sync") }

  /// Drain the queued notification candidates (the host shows them once).
  public func takeNotificationCandidates() -> [NodRequest] {
    defer { notificationCandidates.removeAll() }
    return notificationCandidates
  }

  public func takeRemovedNotificationRequestIds() -> [String] {
    defer { removedNotificationRequestIds.removeAll() }
    return removedNotificationRequestIds
  }

  // MARK: - Internals

  @discardableResult
  private func call(_ method: String, _ params: [String: Any] = [:]) async throws -> Data? {
    guard let client else { throw NodRuntimeError.notStarted }
    let body: [String: Any] = ["id": UUID().uuidString, "method": method, "params": params]
    let requestData = try JSONSerialization.data(withJSONObject: body)
    let responseJSON = await client.request(requestJson: String(decoding: requestData, as: UTF8.self))
    let response = try JSONDecoder().decode(RpcResponseEnvelope.self, from: Data(responseJSON.utf8))
    guard response.ok else {
      throw NodRuntimeError.rpc(response.error ?? "request failed")
    }
    return response.result?.data
  }

  /// Apply the `ClientState` an RPC returned directly, rather than waiting for
  /// the async observer event. The state-mutating RPCs (`enroll`, `refresh`,
  /// `select_server`, …) return the new `ClientState` as their result — applying
  /// it here makes the UI update synchronously with the call (mirroring the old
  /// hand-written client) instead of depending on the event pipeline landing.
  private func applyStateResult(_ data: Data?) {
    guard let data else { return }
    do {
      state = try JSONDecoder.nod.decode(NodRuntimeState.self, from: data)
    } catch {
      // Surface rather than swallow — a decode failure here is a real drift bug.
      lastTransientError = "Failed to decode client state: \(error)"
    }
  }

  private func apply(_ message: NodRuntimeMessage) {
    switch message {
    case .ready(let statePath):
      self.statePath = statePath
    case .state(let state):
      self.state = state
    case .notificationCandidate(let request):
      notificationCandidates.append(request)
    case .notificationRemoved(let requestId):
      removedNotificationRequestIds.append(requestId)
    case .syncStatus:
      // Reflected in the next `state` event's `is_sync_connected`.
      break
    case .authRevoked:
      authRevoked = true
    case .resyncRequired:
      Task { try? await refresh() }
    case .transientError(let message):
      lastTransientError = message
    }
  }

  private struct RpcResponseEnvelope: Decodable {
    let ok: Bool
    let error: String?
    let result: RawJSON?
  }
}

/// Adapts the off-main-thread `NodClientObserver` callback into a decoded,
/// main-actor-bound stream. UniFFI invokes `onMessage` from Rust's pump thread;
/// we decode and hop to the main actor in the closure.
private final class ObserverBridge: NodClientObserver, @unchecked Sendable {
  private let onDecoded: @Sendable (NodRuntimeMessage) -> Void

  init(onDecoded: @escaping @Sendable (NodRuntimeMessage) -> Void) {
    self.onDecoded = onDecoded
  }

  func onMessage(message: String) {
    do {
      onDecoded(try NodRuntimeMessage(from: Data(message.utf8)))
    } catch {
      // Never silently swallow — surface as a transient error so drift is visible
      // instead of producing a "nothing happened" dead end.
      onDecoded(.transientError(message: "runtime event decode failed: \(error)"))
    }
  }
}
