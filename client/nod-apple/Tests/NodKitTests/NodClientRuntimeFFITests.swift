import XCTest

@testable import NodClientFFI

/// Proves the async `nod-client-core` runtime is drivable from Swift through
/// UniFFI: the foreign `NodClientObserver` callback receives serialized events
/// from Rust's detached outbox pump, and the async `request`/`start` methods
/// round-trip. This is the Swift-side confirmation that the tokio↔Swift async
/// bridge + foreign-trait callback work end to end (not just that they compile).
final class NodClientRuntimeFFITests: XCTestCase {
  /// Collects the JSON envelopes Rust pushes to the observer.
  private final class RecordingObserver: NodClientObserver, @unchecked Sendable {
    let lock = NSLock()
    private var _messages: [String] = []

    func onMessage(message: String) {
      lock.lock()
      _messages.append(message)
      lock.unlock()
    }

    var messages: [String] {
      lock.lock(); defer { lock.unlock() }
      return _messages
    }
  }

  /// A signer the `state` RPC never invokes; present only to satisfy the
  /// constructor. The real Secure Enclave signer is `NodKit`'s job.
  private final class UnusedSigner: NodDeviceSigner {
    func provision(profileId: String) throws -> NodDeviceKey {
      throw SignerCallbackError.Failed(message: "unused")
    }
    func signingKey(profileId: String) throws -> NodDeviceKey? { nil }
    func sign(profileId: String, payload: Data) throws -> String {
      throw SignerCallbackError.Failed(message: "unused")
    }
    func remove(profileId: String) throws {}
  }

  override func setUp() {
    super.setUp()
    // Hermetic: file-backed store in a temp dir, no Keychain.
    let dir = NSTemporaryDirectory() + "nod-ffi-swift-\(ProcessInfo.processInfo.processIdentifier)"
    try? FileManager.default.createDirectory(
      atPath: dir, withIntermediateDirectories: true)
    setenv("NOD_CLIENT_CORE_STATE_DIR", dir, 1)
    setenv("NOD_CLIENT_CORE_INSECURE_TOKEN_STORE", "1", 1)
  }

  func testRuntimeStartsAndRoundTripsStateRPCFromSwift() async throws {
    let observer = RecordingObserver()
    let client = try await NodClient(observer: observer, signer: UnusedSigner())
    await client.start()

    // The pump runs on a detached Rust task; poll briefly for the start-up
    // ready + state envelopes.
    for _ in 0..<50 {
      if observer.messages.count >= 2 { break }
      try await Task.sleep(nanoseconds: 10_000_000)
    }

    let seen = observer.messages
    XCTAssertTrue(
      seen.contains { $0.contains("\"kind\":\"ready\"") },
      "expected a ready event, saw \(seen)")
    XCTAssertTrue(
      seen.contains { $0.contains("\"kind\":\"state\"") },
      "expected a state event, saw \(seen)")

    let response = await client.request(
      requestJson: #"{"id":"r1","method":"state","params":{}}"#)
    XCTAssertTrue(response.contains("\"ok\":true"), "state rpc failed: \(response)")
    XCTAssertTrue(response.contains("\"id\":\"r1\""))

    let bad = await client.request(requestJson: "not json")
    XCTAssertTrue(bad.contains("\"ok\":false"))
    XCTAssertTrue(bad.contains("invalid rpc request json"))
  }
}
