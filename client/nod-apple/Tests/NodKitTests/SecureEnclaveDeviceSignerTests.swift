import CryptoKit
import Foundation
import XCTest

import NodClientFFI

@testable import NodKit

/// Proves the SE signer adapter satisfies the runtime's `NodDeviceSigner`
/// contract end to end: a signature it produces over canonical payload bytes
/// **verifies through the Rust verify path** (`NodClientFFI.verifyPayload`, i.e.
/// `nod_proto::verify_payload`). Uses a software P-256 key in place of the
/// Secure Enclave so the test runs hermetically on any host while still
/// exercising real ECDSA — the only difference in production is where the
/// private key lives.
final class SecureEnclaveDeviceSignerTests: XCTestCase {
  func testSignerProducesSignatureVerifiableByRust() throws {
    let keychain = MemoryKeychain()
    let provider = SoftwareP256Provider()
    let store = NodSigningKeyStore(keychain: keychain, signingKeys: provider)
    let signer = SecureEnclaveDeviceSigner(store: store)

    let profileId = "https-nod-example-test"

    // Before enrollment there is no key.
    XCTAssertNil(try signer.signingKey(profileId: profileId))

    // Provision mints a key and returns its public identity.
    let provisioned = try signer.provision(profileId: profileId)
    XCTAssertFalse(provisioned.keyId.isEmpty)
    XCTAssertFalse(provisioned.publicKey.isEmpty)

    // It is now the resolvable signing key.
    let resolved = try XCTUnwrap(try signer.signingKey(profileId: profileId))
    XCTAssertEqual(resolved.keyId, provisioned.keyId)
    XCTAssertEqual(resolved.publicKey, provisioned.publicKey)

    // Sign canonical payload bytes (as the runtime would hand us) and verify the
    // result against the public key through Rust.
    let payload = "nod-decision-v1\nrequest_id:request-1\nkey_id:\(provisioned.keyId)\n"
    let signature = try signer.sign(profileId: profileId, payload: Data(payload.utf8))

    XCTAssertNoThrow(
      try NodClientFFI.verifyPayload(
        publicKey: provisioned.publicKey,
        payload: payload,
        signature: signature),
      "the SE-path signature must verify through nod_proto")

    // A different payload must NOT verify with that signature.
    XCTAssertThrowsError(
      try NodClientFFI.verifyPayload(
        publicKey: provisioned.publicKey,
        payload: payload + "tampered",
        signature: signature))

    // Remove drops the key.
    try signer.remove(profileId: profileId)
    XCTAssertNil(try signer.signingKey(profileId: profileId))
  }
}

// MARK: - Hermetic test doubles

private final class MemoryKeychain: NodKeychainStoring {
  private var values: [String: String] = [:]
  func save(_ value: String, account: String) throws { values[account] = value }
  func load(account: String) throws -> String? { values[account] }
  func delete(account: String) throws { values.removeValue(forKey: account) }
}

/// A software P-256 key standing in for the Secure Enclave: real ECDSA, real
/// x9.63 public key, restorable from `rawRepresentation`.
private struct SoftwareP256Provider: NodSecureEnclaveSigningKeyProvider {
  func generate() throws -> any NodSecureEnclaveSigningPrivateKey {
    SoftwareP256Key(privateKey: P256.Signing.PrivateKey())
  }
  func restore(dataRepresentation: Data) throws -> any NodSecureEnclaveSigningPrivateKey {
    SoftwareP256Key(privateKey: try P256.Signing.PrivateKey(rawRepresentation: dataRepresentation))
  }
}

private struct SoftwareP256Key: NodSecureEnclaveSigningPrivateKey {
  let privateKey: P256.Signing.PrivateKey
  var dataRepresentation: Data { privateKey.rawRepresentation }
  var publicKeyX963Representation: Data { privateKey.publicKey.x963Representation }
  func signatureDER(for data: Data) throws -> Data {
    try privateKey.signature(for: data).derRepresentation
  }
}
