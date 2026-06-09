import Foundation
import NodClientFFI

/// Bridges the shared Rust runtime's `NodDeviceSigner` capability port to
/// NodKit's Secure Enclave key store.
///
/// This is the keystone of "Apple apps onto nod-client-core": the entire client
/// (API, sync, state, decision orchestration) lives in Rust, but the signing key
/// is a non-exportable Secure Enclave key that must never leave the device. So
/// the runtime calls out here — Rust builds the canonical decision payload
/// (`build_decision_signature`) and this signs those exact bytes in hardware.
/// Keyed by server profile id; each enrolled server has its own SE key.
public final class SecureEnclaveDeviceSigner: NodDeviceSigner, @unchecked Sendable {
  private let store: NodSigningKeyStore

  public init(store: NodSigningKeyStore = NodSigningKeyStore()) {
    self.store = store
  }

  public func provision(profileId: String) throws -> NodDeviceKey {
    try mapErrors {
      // `signingKey` is load-or-create: enrolling a new server mints the SE key.
      let key = try store.signingKey(account: account(for: profileId))
      return NodDeviceKey(keyId: key.keyId, publicKey: key.publicKey)
    }
  }

  public func signingKey(profileId: String) throws -> NodDeviceKey? {
    try mapErrors {
      guard let key = store.existingSigningKey(account: account(for: profileId)) else {
        return nil
      }
      return NodDeviceKey(keyId: key.keyId, publicKey: key.publicKey)
    }
  }

  public func sign(profileId: String, payload: Data) throws -> String {
    try mapErrors {
      try store.signPayload(payload, account: account(for: profileId))
    }
  }

  public func remove(profileId: String) throws {
    try mapErrors {
      try store.delete(account: account(for: profileId))
    }
  }

  /// Keychain account namespace for a server profile's decision-signing key.
  private func account(for profileId: String) -> String {
    "decisionSigningKey.\(profileId)"
  }

  /// The Rust side declares a `SignerCallbackError`; any Swift/Keychain/Secure
  /// Enclave failure is surfaced through it so the runtime gets a clean error
  /// rather than an opaque "unexpected" callback failure.
  private func mapErrors<T>(_ body: () throws -> T) throws -> T {
    do {
      return try body()
    } catch let error as SignerCallbackError {
      throw error
    } catch {
      throw SignerCallbackError.Failed(
        message: (error as? LocalizedError)?.errorDescription ?? "\(error)")
    }
  }
}
