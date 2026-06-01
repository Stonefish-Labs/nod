import CryptoKit
#if canImport(DeviceCheck)
import DeviceCheck
#endif
import Foundation

@MainActor
public protocol NodAppAttestationProviding {
  var isSupported: Bool { get }

  func enrollmentAttestation(for request: NodAppAttestationRequest) async throws
    -> NodDeviceAttestation?

  func delete(account: String) throws
}

public enum NodAppAttestationProvider {
  public static let appleAppAttest = "apple_app_attest"
}

@MainActor
public final class NodAppAttestationStore: NodAppAttestationProviding {
  public static let provider = NodAppAttestationProvider.appleAppAttest

  private let keychain: NodKeychainStore

  public init(keychain: NodKeychainStore = NodKeychainStore()) {
    self.keychain = keychain
  }

  public var isSupported: Bool {
    #if canImport(DeviceCheck)
    if #available(iOS 14.0, macOS 11.0, watchOS 9.0, *) {
      return DCAppAttestService.shared.isSupported
    }
    #endif
    return false
  }

  public func enrollmentAttestation(for request: NodAppAttestationRequest) async throws
    -> NodDeviceAttestation?
  {
    guard isSupported else {
      return nil
    }

    #if canImport(DeviceCheck)
    if #available(iOS 14.0, macOS 11.0, watchOS 9.0, *) {
      let service = DCAppAttestService.shared
      let keyId = try await loadOrCreateKey(account: request.account, service: service)
      let clientDataHash = NodEnrollmentAttestation.clientDataHash(
        code: request.code,
        deviceName: request.deviceName,
        platform: request.platform,
        pushProvider: request.pushProvider,
        pushToken: request.pushToken,
        signingKey: request.signingKey,
        appAttestKeyId: keyId
      )
      let attestationObject = try await service.attestKey(keyId, clientDataHash: clientDataHash)
      return NodDeviceAttestation(
        provider: Self.provider,
        keyId: keyId,
        attestationObject: attestationObject.base64URLEncodedString()
      )
    }
    #endif
    return nil
  }

  public func delete(account: String) throws {
    try keychain.delete(account: account)
  }

  #if canImport(DeviceCheck)
  @available(iOS 14.0, macOS 11.0, watchOS 9.0, *)
  private func loadOrCreateKey(account: String, service: DCAppAttestService) async throws -> String {
    if let existing = try keychain.load(account: account) {
      return existing
    }
    let keyId = try await service.generateKey()
    try keychain.save(keyId, account: account)
    return keyId
  }
  #endif
}

public enum NodEnrollmentAttestation {
  /// The server independently builds this exact text before verifying the App Attest object.
  /// Keep every field name and newline stable unless the server contract changes too.
  public static func clientData(
    code: String,
    deviceName: String,
    platform: NodDevicePlatform,
    pushProvider: String?,
    pushToken: String?,
    signingKey: NodDeviceSigningKey,
    appAttestKeyId: String
  ) -> String {
    [
      "nod-enrollment-v1",
      "code_sha256:\(sha256Hex(code.trimmingCharacters(in: .whitespacesAndNewlines)))",
      "device_name_sha256:\(sha256Hex(deviceName.trimmingCharacters(in: .whitespacesAndNewlines)))",
      "platform:\(platform.rawValue)",
      "push_provider:\(normalized(pushProvider) ?? "")",
      "push_token_sha256:\(normalized(pushToken).map(sha256Hex) ?? "")",
      "signing_key_id:\(signingKey.keyId.trimmingCharacters(in: .whitespacesAndNewlines))",
      "signing_key_algorithm:\(signingKey.algorithm)",
      "signing_public_key_sha256:\(sha256Hex(signingKey.publicKey.trimmingCharacters(in: .whitespacesAndNewlines)))",
      "attestation_provider:\(NodAppAttestationProvider.appleAppAttest)",
      "attestation_key_id:\(appAttestKeyId.trimmingCharacters(in: .whitespacesAndNewlines))",
      "",
    ].joined(separator: "\n")
  }

  public static func clientDataHash(
    code: String,
    deviceName: String,
    platform: NodDevicePlatform,
    pushProvider: String?,
    pushToken: String?,
    signingKey: NodDeviceSigningKey,
    appAttestKeyId: String
  ) -> Data {
    Data(
      SHA256.hash(
        data: Data(
          clientData(
            code: code,
            deviceName: deviceName,
            platform: platform,
            pushProvider: pushProvider,
            pushToken: pushToken,
            signingKey: signingKey,
            appAttestKeyId: appAttestKeyId
          ).utf8
        )
      )
    )
  }

  private static func normalized(_ value: String?) -> String? {
    value?.trimmingCharacters(in: .whitespacesAndNewlines).nilIfEmpty
  }

  private static func sha256Hex(_ value: String) -> String {
    SHA256.hash(data: Data(value.utf8)).map { String(format: "%02x", $0) }.joined()
  }
}

private extension String {
  var nilIfEmpty: String? {
    isEmpty ? nil : self
  }
}

private extension Data {
  func base64URLEncodedString() -> String {
    base64EncodedString()
      .replacingOccurrences(of: "+", with: "-")
      .replacingOccurrences(of: "/", with: "_")
      .replacingOccurrences(of: "=", with: "")
  }
}
