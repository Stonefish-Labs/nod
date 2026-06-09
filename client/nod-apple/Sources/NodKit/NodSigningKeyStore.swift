import CryptoKit
import Foundation

public enum NodSigningError: Error, LocalizedError {
    case missingDeviceIdentity
    case missingRequestDigest
    case missingSigningKey
    case secureEnclaveUnavailable

    public var errorDescription: String? {
        switch self {
        case .missingDeviceIdentity:
            return "This device is missing its Nod identity."
        case .missingRequestDigest:
            return "This request cannot be signed because it is missing a server digest."
        case .missingSigningKey:
            return "This device is missing its decision signing key."
        case .secureEnclaveUnavailable:
            return "Secure Enclave is required for Nod decision signing on this device."
        }
    }
}

public struct NodStoredSigningKey: Codable, Hashable, Sendable {
    public let keyId: String
    public let secureEnclaveKey: String

    public init(keyId: String, secureEnclaveKey: String) {
        self.keyId = keyId
        self.secureEnclaveKey = secureEnclaveKey
    }
}

public final class NodSigningKeyStore {
    public static let algorithm = "p256_ecdsa_sha256"

    private let keychain: any NodKeychainStoring
    private let signingKeys: any NodSecureEnclaveSigningKeyProvider

    public init(keychain: NodKeychainStore = NodKeychainStore()) {
        self.keychain = keychain
        self.signingKeys = CryptoKitSecureEnclaveSigningKeyProvider()
    }

    init(
        keychain: any NodKeychainStoring,
        signingKeys: any NodSecureEnclaveSigningKeyProvider
    ) {
        self.keychain = keychain
        self.signingKeys = signingKeys
    }

    public func signingKey(account: String) throws -> NodDeviceSigningKey {
        let stored = try loadOrCreate(account: account)
        let privateKey = try restorePrivateKey(from: stored)
        return NodDeviceSigningKey(
            keyId: stored.keyId,
            algorithm: Self.algorithm,
            publicKey: privateKey.publicKeyX963Representation.base64URLEncodedString()
        )
    }

    public func sign(_ request: NodDecisionSigningRequest) throws -> NodDecisionSignature {
        guard let requestDigest = request.request.requestDigest else {
            throw NodSigningError.missingRequestDigest
        }
        guard let userId = request.userId, let deviceId = request.deviceId else {
            throw NodSigningError.missingDeviceIdentity
        }
        let stored = try load(account: request.account)
        let privateKey = try restorePrivateKey(from: stored)
        let nonce = UUID().uuidString.lowercased()
        let signedAt = Self.iso8601Milliseconds(Date())
        let normalizedText = request.text?
            .trimmingCharacters(in: .whitespacesAndNewlines)
            .nilIfEmpty
        let payload = Self.decisionSigningPayload(
            request: request.request,
            option: request.option,
            text: normalizedText,
            userId: userId,
            deviceId: deviceId,
            keyId: stored.keyId,
            nonce: nonce,
            signedAt: signedAt,
            requestDigest: requestDigest
        )
        let signature = try privateKey.signatureDER(for: Data(payload.utf8)).base64URLEncodedString()
        return NodDecisionSignature(
            keyId: stored.keyId,
            algorithm: Self.algorithm,
            nonce: nonce,
            signedAt: signedAt,
            requestDigest: requestDigest,
            signature: signature
        )
    }

    public func delete(account: String) throws {
        try keychain.delete(account: account)
    }

    private func loadOrCreate(account: String) throws -> NodStoredSigningKey {
        do {
            let existing = try load(account: account)
            return existing
        } catch NodSigningError.missingSigningKey {
            return try create(account: account)
        }
    }

    private func create(account: String) throws -> NodStoredSigningKey {
        let privateKey = try signingKeys.generate()
        // Secure Enclave keys expose a restorable key reference, not raw private-key bytes.
        let stored = NodStoredSigningKey(
            keyId: UUID().uuidString.lowercased(),
            secureEnclaveKey: privateKey.dataRepresentation.base64URLEncodedString()
        )
        let data = try JSONEncoder().encode(stored)
        try keychain.save(data.base64EncodedString(), account: account)
        return stored
    }

    private func load(account: String) throws -> NodStoredSigningKey {
        guard let encoded = try keychain.load(account: account),
              let data = Data(base64Encoded: encoded) else {
            throw NodSigningError.missingSigningKey
        }
        return try JSONDecoder().decode(NodStoredSigningKey.self, from: data)
    }

    private func restorePrivateKey(from stored: NodStoredSigningKey) throws -> any NodSecureEnclaveSigningPrivateKey {
        let keyData = try Data(base64URLEncoded: stored.secureEnclaveKey)
        return try signingKeys.restore(dataRepresentation: keyData)
    }

    private static func decisionSigningPayload(
        request: NodRequest,
        option: NodRequestOption,
        text: String?,
        userId: String,
        deviceId: String,
        keyId: String,
        nonce: String,
        signedAt: String,
        requestDigest: String
    ) -> String {
        [
            "nod-decision-v1",
            "request_id:\(request.id)",
            "request_digest:\(requestDigest)",
            "option_id:\(option.id)",
            "option_kind:\(option.kind.rawValue)",
            "user_id:\(userId)",
            "device_id:\(deviceId)",
            "key_id:\(keyId)",
            "nonce:\(nonce)",
            "signed_at:\(signedAt)",
            "text_sha256:\(sha256Hex(text ?? ""))",
            ""
        ].joined(separator: "\n")
    }

    private static func sha256Hex(_ value: String) -> String {
        SHA256.hash(data: Data(value.utf8)).map { String(format: "%02x", $0) }.joined()
    }

    private static func iso8601Milliseconds(_ date: Date) -> String {
        let formatter = DateFormatter()
        formatter.locale = Locale(identifier: "en_US_POSIX")
        formatter.timeZone = TimeZone(secondsFromGMT: 0)
        formatter.dateFormat = "yyyy-MM-dd'T'HH:mm:ss.SSS'Z'"
        return formatter.string(from: date)
    }
}

protocol NodKeychainStoring: AnyObject {
    func save(_ value: String, account: String) throws
    func load(account: String) throws -> String?
    func delete(account: String) throws
}

extension NodKeychainStore: NodKeychainStoring {}

protocol NodSecureEnclaveSigningKeyProvider {
    func generate() throws -> any NodSecureEnclaveSigningPrivateKey
    func restore(dataRepresentation: Data) throws -> any NodSecureEnclaveSigningPrivateKey
}

protocol NodSecureEnclaveSigningPrivateKey {
    var dataRepresentation: Data { get }
    var publicKeyX963Representation: Data { get }

    func signatureDER(for data: Data) throws -> Data
}

private struct CryptoKitSecureEnclaveSigningKeyProvider: NodSecureEnclaveSigningKeyProvider {
    func generate() throws -> any NodSecureEnclaveSigningPrivateKey {
        guard SecureEnclave.isAvailable else {
            throw NodSigningError.secureEnclaveUnavailable
        }
        return CryptoKitSecureEnclaveSigningPrivateKey(
            privateKey: try SecureEnclave.P256.Signing.PrivateKey()
        )
    }

    func restore(dataRepresentation: Data) throws -> any NodSecureEnclaveSigningPrivateKey {
        guard SecureEnclave.isAvailable else {
            throw NodSigningError.secureEnclaveUnavailable
        }
        return CryptoKitSecureEnclaveSigningPrivateKey(
            privateKey: try SecureEnclave.P256.Signing.PrivateKey(dataRepresentation: dataRepresentation)
        )
    }
}

private struct CryptoKitSecureEnclaveSigningPrivateKey: NodSecureEnclaveSigningPrivateKey {
    let privateKey: SecureEnclave.P256.Signing.PrivateKey

    var dataRepresentation: Data {
        privateKey.dataRepresentation
    }

    var publicKeyX963Representation: Data {
        privateKey.publicKey.x963Representation
    }

    func signatureDER(for data: Data) throws -> Data {
        try privateKey.signature(for: data).derRepresentation
    }
}

private extension String {
    var nilIfEmpty: String? {
        isEmpty ? nil : self
    }
}

private extension Data {
    init(base64URLEncoded value: String) throws {
        var base64 = value.replacingOccurrences(of: "-", with: "+").replacingOccurrences(of: "_", with: "/")
        let padding = (4 - base64.count % 4) % 4
        if padding > 0 {
            base64.append(String(repeating: "=", count: padding))
        }
        guard let data = Data(base64Encoded: base64) else {
            throw NodSigningError.missingSigningKey
        }
        self = data
    }

    func base64URLEncodedString() -> String {
        base64EncodedString()
            .replacingOccurrences(of: "+", with: "-")
            .replacingOccurrences(of: "/", with: "_")
            .replacingOccurrences(of: "=", with: "")
    }
}
