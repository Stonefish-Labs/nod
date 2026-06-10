import Foundation
import XCTest

@testable import NodKit

final class NodSigningKeyStoreTests: XCTestCase {
  func testSigningKeyCreatesSecureEnclaveBackedKey() throws {
    let keychain = TestKeychainStore()
    let key = TestSecureEnclavePrivateKey(
      dataRepresentation: Data("secure-key-1".utf8),
      publicKeyX963Representation: Data("public-key-1".utf8),
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
  let publicKeyX963Representation: Data
  private let signature: Data

  var signedData: [Data] = []

  init(dataRepresentation: Data, publicKeyX963Representation: Data, signature: Data) {
    self.dataRepresentation = dataRepresentation
    self.publicKeyX963Representation = publicKeyX963Representation
    self.signature = signature
  }

  func signatureDER(for data: Data) throws -> Data {
    signedData.append(data)
    return signature
  }
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
