import CryptoKit
import Foundation
import NodClientFFI

public enum NodSigningError: Error, LocalizedError {
    case missingSigningKey
    case secureEnclaveUnavailable

    public var errorDescription: String? {
        switch self {
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

    /// Keychain account namespace for a server profile's decision-signing key.
    /// The single source for this literal — NodStore and the Secure Enclave
    /// signer both key off it.
    public static func account(for profileId: String) -> String {
        "decisionSigningKey.\(profileId)"
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

    /// The existing key for `account`, or nil if none is stored. Unlike
    /// `signingKey(account:)` this never creates a key — used to answer "is this
    /// profile enrolled?" without provisioning.
    public func existingSigningKey(account: String) -> NodDeviceSigningKey? {
        guard let stored = try? load(account: account),
              let privateKey = try? restorePrivateKey(from: stored) else {
            return nil
        }
        return NodDeviceSigningKey(
            keyId: stored.keyId,
            algorithm: Self.algorithm,
            publicKey: privateKey.publicKeyX963Representation.base64URLEncodedString()
        )
    }

    /// Sign already-canonicalized payload bytes with the Secure Enclave key for
    /// `account`. The shared Rust runtime builds the canonical decision payload
    /// (`build_decision_signature`) and hands it here; the only thing that
    /// happens in Swift is the hardware signature. Returns a base64url DER ECDSA
    /// signature.
    public func signPayload(_ payload: Data, account: String) throws -> String {
        let stored = try load(account: account)
        let privateKey = try restorePrivateKey(from: stored)
        return try privateKey.signatureDER(for: payload).base64URLEncodedString()
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
