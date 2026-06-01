import Foundation
import XCTest

@testable import NodKit

final class NodSigningKeyStoreTests: XCTestCase {
  func testSigningKeyCreatesSecureEnclaveBackedKey() throws {
    let keychain = TestKeychainStore()
    let key = TestSecureEnclavePrivateKey(
      dataRepresentation: Data("secure-key-1".utf8),
      publicKeyRawRepresentation: Data("public-key-1".utf8),
      signature: Data("der-signature".utf8)
    )
    let provider = TestSecureEnclaveSigningKeyProvider(generatedKeys: [key])
    let store = NodSigningKeyStore(keychain: keychain, signingKeys: provider)

    let signingKey = try store.signingKey(account: "decisionSigningKey.test")

    XCTAssertEqual(signingKey.algorithm, NodSigningKeyStore.algorithm)
    XCTAssertEqual(signingKey.publicKey, base64URL(Data("public-key-1".utf8)))
    XCTAssertEqual(provider.generateCount, 1)

    let stored = try storedJSON(from: keychain.value(for: "decisionSigningKey.test"))
    XCTAssertNotNil(stored["keyId"])
    XCTAssertEqual(stored["secureEnclaveKey"] as? String, base64URL(Data("secure-key-1".utf8)))
    XCTAssertNil(stored["privateKey"])
  }

  func testSigningKeyDoesNotReplaceInvalidStoredKey() throws {
    let account = "decisionSigningKey.invalid"
    let keychain = TestKeychainStore()
    keychain.values[account] = try keychainValue(for: ["unexpected": "shape"])
    let provider = TestSecureEnclaveSigningKeyProvider()
    let store = NodSigningKeyStore(keychain: keychain, signingKeys: provider)

    XCTAssertThrowsError(try store.signingKey(account: account))
    XCTAssertEqual(provider.generateCount, 0)
    XCTAssertTrue(provider.restoredDataRepresentations.isEmpty)
    XCTAssertNotNil(try storedJSON(from: keychain.value(for: account))["unexpected"])
  }

  func testSigningUsesStoredSecureEnclaveKey() throws {
    let account = "decisionSigningKey.secure"
    let key = TestSecureEnclavePrivateKey(
      dataRepresentation: Data("secure-key-3".utf8),
      publicKeyRawRepresentation: Data("public-key-3".utf8),
      signature: Data("der-signature".utf8)
    )
    let keychain = TestKeychainStore()
    keychain.values[account] = try keychainValue(
      for: NodStoredSigningKey(
        keyId: "secure-key-id",
        secureEnclaveKey: base64URL(key.dataRepresentation)
      )
    )
    let provider = TestSecureEnclaveSigningKeyProvider(restoredKeys: [key.dataRepresentation: key])
    let store = NodSigningKeyStore(keychain: keychain, signingKeys: provider)

    let signature = try store.sign(NodDecisionSigningRequest(
      event: makeEvent(),
      action: NodAction(id: "approve", label: "Approve", kind: .approve),
      text: " approved ",
      userId: "user-1",
      deviceId: "device-1",
      account: account
    ))

    XCTAssertEqual(signature.keyId, "secure-key-id")
    XCTAssertEqual(signature.algorithm, NodSigningKeyStore.algorithm)
    XCTAssertEqual(signature.requestDigest, "digest-1")
    XCTAssertEqual(signature.signature, base64URL(Data("der-signature".utf8)))
    XCTAssertEqual(provider.restoredDataRepresentations, [key.dataRepresentation])

    let payload = try XCTUnwrap(String(data: try XCTUnwrap(key.signedData.first), encoding: .utf8))
    XCTAssertTrue(payload.contains("request_digest:digest-1"))
    XCTAssertTrue(payload.contains("key_id:secure-key-id"))
  }
}

private final class TestKeychainStore: NodKeychainStoring {
  var values: [String: String] = [:]

  func save(_ value: String, account: String) throws {
    values[account] = value
  }

  func load(account: String) throws -> String? {
    values[account]
  }

  func delete(account: String) throws {
    values.removeValue(forKey: account)
  }

  func value(for account: String) throws -> String {
    try XCTUnwrap(values[account])
  }
}

private final class TestSecureEnclaveSigningKeyProvider: NodSecureEnclaveSigningKeyProvider {
  private var generatedKeys: [TestSecureEnclavePrivateKey]
  private var restoredKeys: [Data: TestSecureEnclavePrivateKey]

  var generateCount = 0
  var restoredDataRepresentations: [Data] = []

  init(
    generatedKeys: [TestSecureEnclavePrivateKey] = [],
    restoredKeys: [Data: TestSecureEnclavePrivateKey] = [:]
  ) {
    self.generatedKeys = generatedKeys
    self.restoredKeys = restoredKeys
  }

  func generate() throws -> any NodSecureEnclaveSigningPrivateKey {
    generateCount += 1
    let key = try XCTUnwrap(generatedKeys.isEmpty ? nil : generatedKeys.removeFirst())
    restoredKeys[key.dataRepresentation] = key
    return key
  }

  func restore(dataRepresentation: Data) throws -> any NodSecureEnclaveSigningPrivateKey {
    restoredDataRepresentations.append(dataRepresentation)
    return try XCTUnwrap(restoredKeys[dataRepresentation])
  }
}

private final class TestSecureEnclavePrivateKey: NodSecureEnclaveSigningPrivateKey {
  let dataRepresentation: Data
  let publicKeyRawRepresentation: Data
  private let signature: Data

  var signedData: [Data] = []

  init(dataRepresentation: Data, publicKeyRawRepresentation: Data, signature: Data) {
    self.dataRepresentation = dataRepresentation
    self.publicKeyRawRepresentation = publicKeyRawRepresentation
    self.signature = signature
  }

  func signatureDER(for data: Data) throws -> Data {
    signedData.append(data)
    return signature
  }
}

private func makeEvent() throws -> NodEvent {
  let data = """
    {
      "id": "event-1",
      "channel_id": "channel-1",
      "recipients": [],
      "action_resolution": "shared",
      "title": "Deploy?",
      "summary": "Production deploy",
      "body_markdown": "Approve deploy",
      "fields": [],
      "links": [],
      "image_url": null,
      "priority": 1,
      "privacy": "normal",
      "dedupe_key": null,
      "expires_at": null,
      "status": "pending",
      "created_at": "2026-05-31T12:00:00.000Z",
      "updated_at": "2026-05-31T12:00:00.000Z",
      "resolved_at": null,
      "result": null,
      "user_results": [],
      "callback_url": null,
      "request_digest": "digest-1",
      "actions": []
    }
    """.data(using: .utf8)!
  return try JSONDecoder.nod.decode(NodEvent.self, from: data)
}

private func keychainValue<Value: Encodable>(for value: Value) throws -> String {
  try JSONEncoder().encode(value).base64EncodedString()
}

private func storedJSON(from value: String) throws -> [String: Any] {
  let data = try XCTUnwrap(Data(base64Encoded: value))
  let json = try JSONSerialization.jsonObject(with: data)
  return try XCTUnwrap(json as? [String: Any])
}

private func base64URL(_ data: Data) -> String {
  data.base64EncodedString()
    .replacingOccurrences(of: "+", with: "-")
    .replacingOccurrences(of: "/", with: "_")
    .replacingOccurrences(of: "=", with: "")
}
