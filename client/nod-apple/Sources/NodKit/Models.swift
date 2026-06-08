import Foundation

public enum NodDevicePlatform: String, Codable, Sendable {
  case ios
  case macos
  case watchos
  case windows
  case linux
  case unknown
}

public struct NodSource: Codable, Identifiable, Hashable, Sendable {
  public var id: String
  public var name: String
  public var emoji: String
  public var subscribed: Bool
  public var createdAt: Date

  enum CodingKeys: String, CodingKey {
    case id, name, emoji, subscribed
    case createdAt = "created_at"
  }

  public init(from decoder: Decoder) throws {
    let container = try decoder.container(keyedBy: CodingKeys.self)
    id = try container.decode(String.self, forKey: .id)
    name = try container.decode(String.self, forKey: .name)
    emoji = try container.decode(String.self, forKey: .emoji)
    subscribed = try container.decodeIfPresent(Bool.self, forKey: .subscribed) ?? true
    createdAt = try container.decode(Date.self, forKey: .createdAt)
  }
}

public struct NodServerProfile: Codable, Identifiable, Hashable, Sendable {
  public var id: String
  public var name: String
  public var baseURLString: String
  public var deviceName: String
  public var deviceId: String?
  public var userId: String?
  public var userName: String?

  public init(
    id: String,
    name: String,
    baseURLString: String,
    deviceName: String,
    deviceId: String? = nil,
    userId: String? = nil,
    userName: String? = nil
  ) {
    self.id = id
    self.name = name
    self.baseURLString = baseURLString
    self.deviceName = deviceName
    self.deviceId = deviceId
    self.userId = userId
    self.userName = userName
  }
}

public struct NodUser: Codable, Identifiable, Hashable, Sendable {
  public let id: String
  public let name: String
  public let createdAt: Date
  public let updatedAt: Date

  enum CodingKeys: String, CodingKey {
    case id, name
    case createdAt = "created_at"
    case updatedAt = "updated_at"
  }
}

public struct NodUserDevice: Codable, Identifiable, Hashable, Sendable {
  public let id: String
  public let userId: String
  public var name: String
  public let platform: NodDevicePlatform
  public let nativeAppId: String?
  public let pushProvider: String?
  public let hasPushToken: Bool
  public let hasSigningKey: Bool
  public let notificationSound: String
  public let attestation: NodDeviceAttestationSummary?
  public let lastSeenAt: Date
  public let createdAt: Date
  public let isCurrent: Bool

  enum CodingKeys: String, CodingKey {
    case id, name, platform
    case userId = "user_id"
    case nativeAppId = "native_app_id"
    case pushProvider = "push_provider"
    case hasPushToken = "has_push_token"
    case hasSigningKey = "has_signing_key"
    case notificationSound = "notification_sound"
    case attestation
    case lastSeenAt = "last_seen_at"
    case createdAt = "created_at"
    case isCurrent = "is_current"
  }
}

public enum NodDeviceAttestationStatus: String, Codable, Sendable {
  case verified
  case failed
}

public struct NodDeviceAttestationSummary: Codable, Hashable, Sendable {
  public let provider: String
  public let status: NodDeviceAttestationStatus
  public let keyId: String?
  public let teamId: String?
  public let bundleId: String?
  public let environment: String?
  public let verifiedAt: Date?
  public let failureReason: String?

  enum CodingKeys: String, CodingKey {
    case provider, status, environment
    case keyId = "key_id"
    case teamId = "team_id"
    case bundleId = "bundle_id"
    case verifiedAt = "verified_at"
    case failureReason = "failure_reason"
  }
}

public struct NodNotificationSoundOption: Identifiable, Hashable, Sendable {
  public let id: String
  public let label: String

  public init(id: String, label: String) {
    self.id = id
    self.label = label
  }
}

public enum NodNotificationDeliveryMode: String, Codable, Sendable {
  case push
  case websocket
}

public struct NodNotificationDelivery: Codable, Hashable, Sendable {
  public let mode: NodNotificationDeliveryMode

  public init(mode: NodNotificationDeliveryMode) {
    self.mode = mode
  }
}

public struct NodField: Codable, Hashable, Sendable {
  public let label: String
  public let value: String
  public let style: String?
}

public struct NodLink: Codable, Hashable, Sendable {
  public let label: String
  public let url: String
}

public enum NodOptionKind: String, Codable, Sendable {
  case approve
  case approveWithText = "approve_with_text"
  case reject
  case rejectWithText = "reject_with_text"
  case dismiss
  case open
  case custom

  public init(from decoder: Decoder) throws {
    let container = try decoder.singleValueContainer()
    let value = try container.decode(String.self)
    self = NodOptionKind(rawValue: value) ?? .custom
  }
}

public struct NodRequestOption: Codable, Identifiable, Hashable, Sendable {
  public let id: String
  public let label: String
  public let kind: NodOptionKind
  public let style: String
  public let requiresText: Bool
  public let textPlaceholder: String?
  public let destructive: Bool
  public let foreground: Bool

  public init(
    id: String,
    label: String,
    kind: NodOptionKind,
    style: String = "default",
    requiresText: Bool = false,
    textPlaceholder: String? = nil,
    destructive: Bool = false,
    foreground: Bool = false
  ) {
    self.id = id
    self.label = label
    self.kind = kind
    self.style = style
    self.requiresText = requiresText
    self.textPlaceholder = textPlaceholder
    self.destructive = destructive
    self.foreground = foreground
  }

  enum CodingKeys: String, CodingKey {
    case id, label, kind, style, destructive, foreground
    case requiresText = "requires_text"
    case textPlaceholder = "text_placeholder"
  }
}

public enum NodRequestStatus: String, Codable, Sendable {
  case pending
  case resolved
  case expired
  case cancelled
}

public struct NodDecision: Codable, Hashable, Sendable {
  public let requestId: String
  public let optionId: String
  public let optionKind: NodOptionKind
  public let optionLabel: String
  public let text: String?
  public let actorUserId: String?
  public let actorDeviceId: String?
  public let signature: NodDecisionSignatureRecord?
  public let resolvedAt: Date

  enum CodingKeys: String, CodingKey {
    case text, signature
    case requestId = "request_id"
    case optionId = "option_id"
    case optionKind = "option_kind"
    case optionLabel = "option_label"
    case actorUserId = "actor_user_id"
    case actorDeviceId = "actor_device_id"
    case resolvedAt = "resolved_at"
  }
}

public struct NodDecisionSignatureRecord: Codable, Hashable, Sendable {
  public let keyId: String
  public let algorithm: String
  public let nonce: String
  public let signedAt: String
  public let requestDigest: String
  public let signingPayload: String
  public let signature: String
  public let verified: Bool

  enum CodingKeys: String, CodingKey {
    case algorithm, nonce, signature, verified
    case keyId = "key_id"
    case signedAt = "signed_at"
    case requestDigest = "request_digest"
    case signingPayload = "signing_payload"
  }
}

public struct NodUserDecision: Codable, Hashable, Sendable {
  public let userId: String
  public let decision: NodDecision

  enum CodingKeys: String, CodingKey {
    case decision
    case userId = "user_id"
  }
}

public enum NodDecisionResolution: String, Codable, Sendable {
  case shared
  case perUser = "per_user"
}

public struct NodRequestNotification: Codable, Hashable, Sendable {
  public let redact: Bool
  public let title: String?
  public let body: String?

  enum CodingKeys: String, CodingKey {
    case redact, title, body
  }

  public init(redact: Bool = false, title: String? = nil, body: String? = nil) {
    self.redact = redact
    self.title = title
    self.body = body
  }

  public init(from decoder: Decoder) throws {
    let container = try decoder.container(keyedBy: CodingKeys.self)
    redact = try container.decodeIfPresent(Bool.self, forKey: .redact) ?? false
    title = try container.decodeIfPresent(String.self, forKey: .title)
    body = try container.decodeIfPresent(String.self, forKey: .body)
  }
}

public struct NodRequest: Codable, Identifiable, Hashable, Sendable {
  public let id: String
  public let requestId: String
  public let sourceId: String
  public let recipients: [String]
  public let decisionResolution: NodDecisionResolution
  public let title: String
  public let summary: String
  public let bodyMarkdown: String
  public let fields: [NodField]
  public let links: [NodLink]
  public let imageUrl: String?
  public let notification: NodRequestNotification
  public let dedupeKey: String?
  public let expiresAt: Date?
  public let status: NodRequestStatus
  public let createdAt: Date
  public let updatedAt: Date
  public let resolvedAt: Date?
  public let decision: NodDecision?
  public let decisions: [NodUserDecision]
  public let callbackUrl: String?
  public let options: [NodRequestOption]
  public let requestDigest: String?

  enum CodingKeys: String, CodingKey {
    case id, title, summary, fields, links, notification, status, decision, decisions, options
    case requestId = "request_id"
    case sourceId = "source_id"
    case recipients
    case decisionResolution = "decision_resolution"
    case bodyMarkdown = "body_markdown"
    case imageUrl = "image_url"
    case dedupeKey = "dedupe_key"
    case expiresAt = "expires_at"
    case createdAt = "created_at"
    case updatedAt = "updated_at"
    case resolvedAt = "resolved_at"
    case callbackUrl = "callback_url"
    case requestDigest = "request_digest"
  }

  public init(from decoder: Decoder) throws {
    let container = try decoder.container(keyedBy: CodingKeys.self)
    id = try container.decode(String.self, forKey: .id)
    requestId = try container.decode(String.self, forKey: .requestId)
    sourceId = try container.decode(String.self, forKey: .sourceId)
    recipients = try container.decodeIfPresent([String].self, forKey: .recipients) ?? []
    decisionResolution = try container.decodeIfPresent(
      NodDecisionResolution.self,
      forKey: .decisionResolution
    ) ?? .shared
    title = try container.decode(String.self, forKey: .title)
    summary = try container.decode(String.self, forKey: .summary)
    bodyMarkdown = try container.decode(String.self, forKey: .bodyMarkdown)
    fields = try container.decodeIfPresent([NodField].self, forKey: .fields) ?? []
    links = try container.decodeIfPresent([NodLink].self, forKey: .links) ?? []
    imageUrl = try container.decodeIfPresent(String.self, forKey: .imageUrl)
    notification = try container.decode(NodRequestNotification.self, forKey: .notification)
    dedupeKey = try container.decodeIfPresent(String.self, forKey: .dedupeKey)
    expiresAt = try container.decodeIfPresent(Date.self, forKey: .expiresAt)
    status = try container.decode(NodRequestStatus.self, forKey: .status)
    createdAt = try container.decode(Date.self, forKey: .createdAt)
    updatedAt = try container.decode(Date.self, forKey: .updatedAt)
    resolvedAt = try container.decodeIfPresent(Date.self, forKey: .resolvedAt)
    decision = try container.decodeIfPresent(NodDecision.self, forKey: .decision)
    decisions = try container.decodeIfPresent([NodUserDecision].self, forKey: .decisions) ?? []
    callbackUrl = try container.decodeIfPresent(String.self, forKey: .callbackUrl)
    options = try container.decode([NodRequestOption].self, forKey: .options)
    requestDigest = try container.decodeIfPresent(String.self, forKey: .requestDigest)
  }
}

public struct NodDeviceSigningKey: Codable, Hashable, Sendable {
  public let keyId: String
  public let algorithm: String
  public let publicKey: String

  public init(keyId: String, algorithm: String, publicKey: String) {
    self.keyId = keyId
    self.algorithm = algorithm
    self.publicKey = publicKey
  }

  enum CodingKeys: String, CodingKey {
    case algorithm
    case keyId = "key_id"
    case publicKey = "public_key"
  }
}

public struct NodEnrollmentRequest: Encodable, Sendable {
  public let code: String
  public let deviceName: String
  public let platform: NodDevicePlatform
  public let nativeAppId: String?
  public let pushProvider: String?
  public let pushToken: String?
  public let signingKey: NodDeviceSigningKey
  public let attestation: NodDeviceAttestation?

  public init(
    code: String,
    deviceName: String,
    platform: NodDevicePlatform,
    nativeAppId: String? = nil,
    pushProvider: String? = nil,
    pushToken: String? = nil,
    signingKey: NodDeviceSigningKey,
    attestation: NodDeviceAttestation? = nil
  ) {
    self.code = code
    self.deviceName = deviceName
    self.platform = platform
    self.nativeAppId = nativeAppId
    self.pushProvider = pushProvider
    self.pushToken = pushToken
    self.signingKey = signingKey
    self.attestation = attestation
  }

  enum CodingKeys: String, CodingKey {
    case code, platform, attestation
    case deviceName = "device_name"
    case nativeAppId = "native_app_id"
    case pushProvider = "push_provider"
    case pushToken = "push_token"
    case signingKey = "signing_key"
  }
}

public struct NodDeviceAttestation: Codable, Hashable, Sendable {
  public let provider: String
  public let keyId: String
  public let attestationObject: String

  public init(provider: String, keyId: String, attestationObject: String) {
    self.provider = provider
    self.keyId = keyId
    self.attestationObject = attestationObject
  }

  enum CodingKeys: String, CodingKey {
    case provider
    case keyId = "key_id"
    case attestationObject = "attestation_object"
  }
}

public struct NodAppAttestationRequest: Sendable {
  public let code: String
  public let deviceName: String
  public let platform: NodDevicePlatform
  public let pushProvider: String?
  public let pushToken: String?
  public let signingKey: NodDeviceSigningKey
  public let account: String

  public init(
    code: String,
    deviceName: String,
    platform: NodDevicePlatform,
    pushProvider: String?,
    pushToken: String?,
    signingKey: NodDeviceSigningKey,
    account: String
  ) {
    self.code = code
    self.deviceName = deviceName
    self.platform = platform
    self.pushProvider = pushProvider
    self.pushToken = pushToken
    self.signingKey = signingKey
    self.account = account
  }
}

public struct NodDecisionSignature: Codable, Hashable, Sendable {
  public let keyId: String
  public let algorithm: String
  public let nonce: String
  public let signedAt: String
  public let requestDigest: String
  public let signature: String

  enum CodingKeys: String, CodingKey {
    case algorithm, nonce, signature
    case keyId = "key_id"
    case signedAt = "signed_at"
    case requestDigest = "request_digest"
  }
}

public struct NodDecisionSigningRequest: Sendable {
  public let request: NodRequest
  public let option: NodRequestOption
  public let text: String?
  public let userId: String?
  public let deviceId: String?
  public let account: String

  public init(
    request: NodRequest,
    option: NodRequestOption,
    text: String? = nil,
    userId: String?,
    deviceId: String?,
    account: String
  ) {
    self.request = request
    self.option = option
    self.text = text
    self.userId = userId
    self.deviceId = deviceId
    self.account = account
  }
}

public struct NodSyncEnvelope: Decodable, Sendable {
  public let kind: String
  public let at: Date
  public let request: NodRequest?
  public let source: NodSource?
  public let notificationDelivery: NodNotificationDelivery?

  enum CodingKeys: String, CodingKey {
    case kind, at, payload
  }

  enum PayloadKeys: String, CodingKey {
    case request, source
    case notificationDelivery = "notification_delivery"
  }

  public init(from decoder: Decoder) throws {
    let container = try decoder.container(keyedBy: CodingKeys.self)
    kind = try container.decode(String.self, forKey: .kind)
    at = try container.decode(Date.self, forKey: .at)
    if let payload = try? container.nestedContainer(keyedBy: PayloadKeys.self, forKey: .payload) {
      request = try? payload.decode(NodRequest.self, forKey: .request)
      source = try? payload.decode(NodSource.self, forKey: .source)
      notificationDelivery = try? payload.decode(
        NodNotificationDelivery.self, forKey: .notificationDelivery)
    } else {
      request = nil
      source = nil
      notificationDelivery = nil
    }
  }
}

public struct NodRequestQuery: Hashable, Sendable {
  public var sourceId: String?
  public var includeCleared: Bool
  public var limit: Int?

  public static let activeOnly = NodRequestQuery()

  public init(sourceId: String? = nil, includeCleared: Bool = false, limit: Int? = nil) {
    self.sourceId = sourceId
    self.includeCleared = includeCleared
    self.limit = limit
  }
}

public struct EnrollDeviceResponse: Codable, Sendable {
  public let deviceId: String
  public let userId: String
  public let userName: String
  public let token: String
  public let notificationDelivery: NodNotificationDelivery
  public let sources: [NodSource]
  public let devices: [NodUserDevice]

  enum CodingKeys: String, CodingKey {
    case token, sources, devices
    case deviceId = "device_id"
    case userId = "user_id"
    case userName = "user_name"
    case notificationDelivery = "notification_delivery"
  }
}

public struct CurrentUserResponse: Codable, Sendable {
  public let user: NodUser
  public let currentDevice: NodUserDevice
  public let notificationDelivery: NodNotificationDelivery

  enum CodingKeys: String, CodingKey {
    case user
    case currentDevice = "current_device"
    case notificationDelivery = "notification_delivery"
  }
}

public struct UserDevicesResponse: Codable, Sendable {
  public let devices: [NodUserDevice]
}

public struct UserDeviceResponse: Codable, Sendable {
  public let device: NodUserDevice
}

public struct SourcesResponse: Codable, Sendable {
  public let sources: [NodSource]
}

public struct RequestsResponse: Codable, Sendable {
  public let requests: [NodRequest]
}

public struct RequestResponse: Codable, Sendable {
  public let request: NodRequest
}

public struct DecisionResponse: Codable, Sendable {
  public let requestId: String
  public let status: NodRequestStatus
  public let decision: NodDecision?

  enum CodingKeys: String, CodingKey {
    case status, decision
    case requestId = "request_id"
  }
}
