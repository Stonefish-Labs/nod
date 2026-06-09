import Foundation

/// The decoded `ClientState` the shared Rust runtime (`nod-client-core`) emits
/// over the FFI — the same view type the Tauri desktop renders. NodKit decodes
/// it instead of maintaining a parallel store, reusing the wire models
/// (`NodChannel`/`NodUser`/`NodUserDevice`/`NodRequest`) that already match the
/// shared `nod-proto` shapes.
public struct NodRuntimeState: Codable, Sendable {
  public var servers: [NodRuntimeServerProfile]
  public var selectedServerId: String?
  public var currentUser: NodUser?
  public var devices: [NodUserDevice]
  public var channels: [NodChannel]
  public var pendingCountsByChannel: [String: Int]
  public var requests: [NodRequest]
  public var selectedChannelId: String?
  public var selectedRequestId: String?
  public var notificationSound: String
  public var notificationDeliveryMode: NodNotificationDeliveryMode
  public var isRegistered: Bool
  public var isSyncConnected: Bool
  public var lastError: String?

  enum CodingKeys: String, CodingKey {
    case servers, channels, requests, devices
    case selectedServerId = "selected_server_id"
    case currentUser = "current_user"
    case pendingCountsByChannel = "pending_counts_by_channel"
    case selectedChannelId = "selected_channel_id"
    case selectedRequestId = "selected_request_id"
    case notificationSound = "notification_sound"
    case notificationDeliveryMode = "notification_delivery_mode"
    case isRegistered = "is_registered"
    case isSyncConnected = "is_sync_connected"
    case lastError = "last_error"
  }

  public init(from decoder: Decoder) throws {
    let c = try decoder.container(keyedBy: CodingKeys.self)
    servers = try c.decodeIfPresent([NodRuntimeServerProfile].self, forKey: .servers) ?? []
    selectedServerId = try c.decodeIfPresent(String.self, forKey: .selectedServerId)
    currentUser = try c.decodeIfPresent(NodUser.self, forKey: .currentUser)
    devices = try c.decodeIfPresent([NodUserDevice].self, forKey: .devices) ?? []
    channels = try c.decodeIfPresent([NodChannel].self, forKey: .channels) ?? []
    pendingCountsByChannel =
      try c.decodeIfPresent([String: Int].self, forKey: .pendingCountsByChannel) ?? [:]
    requests = try c.decodeIfPresent([NodRequest].self, forKey: .requests) ?? []
    selectedChannelId = try c.decodeIfPresent(String.self, forKey: .selectedChannelId)
    selectedRequestId = try c.decodeIfPresent(String.self, forKey: .selectedRequestId)
    notificationSound = try c.decodeIfPresent(String.self, forKey: .notificationSound) ?? "default"
    notificationDeliveryMode =
      try c.decodeIfPresent(NodNotificationDeliveryMode.self, forKey: .notificationDeliveryMode)
      ?? .websocket
    isRegistered = try c.decodeIfPresent(Bool.self, forKey: .isRegistered) ?? false
    isSyncConnected = try c.decodeIfPresent(Bool.self, forKey: .isSyncConnected) ?? false
    lastError = try c.decodeIfPresent(String.self, forKey: .lastError)
  }

  public var totalPendingCount: Int {
    pendingCountsByChannel.values.reduce(0, +)
  }

  public var subscribedChannels: [NodChannel] {
    channels.filter(\.subscribed)
  }

  public var selectedServer: NodRuntimeServerProfile? {
    guard let selectedServerId else { return servers.first }
    return servers.first { $0.id == selectedServerId } ?? servers.first
  }
}

/// `ClientState.servers` entries — the runtime's `ServerProfile` (snake_case),
/// distinct from NodKit's locally-persisted `NodServerProfile`.
public struct NodRuntimeServerProfile: Codable, Identifiable, Hashable, Sendable {
  public var id: String
  public var name: String
  public var baseUrlString: String
  public var deviceName: String
  public var deviceId: String?
  public var userId: String?
  public var userName: String?

  enum CodingKeys: String, CodingKey {
    case id, name
    case baseUrlString = "base_url_string"
    case deviceName = "device_name"
    case deviceId = "device_id"
    case userId = "user_id"
    case userName = "user_name"
  }
}

/// A decoded `NodClientMessage` envelope (`{ "kind": …, "payload": … }`) the
/// runtime pushes to its observer. Mirrors nod-client-core's `NodClientMessage`.
public enum NodRuntimeMessage: Sendable {
  case ready(statePath: String)
  case state(NodRuntimeState)
  case notificationCandidate(NodRequest)
  case notificationRemoved(requestId: String)
  case syncStatus(connected: Bool)
  case authRevoked
  case resyncRequired
  case transientError(message: String)

  public init(from data: Data) throws {
    // `.nod` carries the ISO-8601 date strategy the wire models require.
    let envelope = try JSONDecoder.nod.decode(Envelope.self, from: data)
    switch envelope.kind {
    case "ready":
      self = .ready(statePath: envelope.payload.decode(ReadyPayload.self)?.statePath ?? "")
    case "state":
      guard let state = envelope.payload.decode(NodRuntimeState.self) else {
        throw NodRuntimeMessageError.malformed("state")
      }
      self = .state(state)
    case "notification_candidate":
      guard let payload = envelope.payload.decode(NotificationCandidatePayload.self) else {
        throw NodRuntimeMessageError.malformed("notification_candidate")
      }
      self = .notificationCandidate(payload.request)
    case "notification_removed":
      self = .notificationRemoved(
        requestId: envelope.payload.decode(NotificationRemovedPayload.self)?.requestId ?? "")
    case "sync_status":
      self = .syncStatus(
        connected: envelope.payload.decode(SyncStatusPayload.self)?.connected ?? false)
    case "auth_revoked":
      self = .authRevoked
    case "resync_required":
      self = .resyncRequired
    case "transient_error":
      self = .transientError(
        message: envelope.payload.decode(TransientErrorPayload.self)?.message ?? "")
    default:
      throw NodRuntimeMessageError.unknownKind(envelope.kind)
    }
  }

  private struct Envelope: Decodable {
    let kind: String
    let payload: RawJSON
  }

  private struct ReadyPayload: Decodable {
    let statePath: String
    enum CodingKeys: String, CodingKey { case statePath = "state_path" }
  }
  private struct NotificationCandidatePayload: Decodable { let request: NodRequest }
  private struct NotificationRemovedPayload: Decodable {
    let requestId: String
    enum CodingKeys: String, CodingKey { case requestId = "request_id" }
  }
  private struct SyncStatusPayload: Decodable { let connected: Bool }
  private struct TransientErrorPayload: Decodable { let message: String }
}

public enum NodRuntimeMessageError: Error {
  case malformed(String)
  case unknownKind(String)
}

/// Holds an arbitrary JSON value so a payload can be decoded lazily into the
/// concrete shape each message kind expects.
struct RawJSON: Decodable {
  let data: Data

  init(from decoder: Decoder) throws {
    let container = try decoder.singleValueContainer()
    if let value = try? container.decode(AnyCodable.self) {
      data = (try? JSONEncoder().encode(value)) ?? Data("null".utf8)
    } else {
      data = Data("null".utf8)
    }
  }

  func decode<T: Decodable>(_ type: T.Type) -> T? {
    // `.nod` so nested wire models (which have ISO-8601 `Date` fields) decode.
    try? JSONDecoder.nod.decode(type, from: data)
  }
}

/// Minimal type-erased JSON for round-tripping an opaque payload.
private struct AnyCodable: Codable {
  let value: Any

  init(from decoder: Decoder) throws {
    let container = try decoder.singleValueContainer()
    if container.decodeNil() {
      value = NSNull()
    } else if let bool = try? container.decode(Bool.self) {
      value = bool
    } else if let int = try? container.decode(Int.self) {
      value = int
    } else if let double = try? container.decode(Double.self) {
      value = double
    } else if let string = try? container.decode(String.self) {
      value = string
    } else if let array = try? container.decode([AnyCodable].self) {
      value = array.map(\.value)
    } else if let dict = try? container.decode([String: AnyCodable].self) {
      value = dict.mapValues(\.value)
    } else {
      value = NSNull()
    }
  }

  func encode(to encoder: Encoder) throws {
    var container = encoder.singleValueContainer()
    switch value {
    case is NSNull: try container.encodeNil()
    case let bool as Bool: try container.encode(bool)
    case let int as Int: try container.encode(int)
    case let double as Double: try container.encode(double)
    case let string as String: try container.encode(string)
    case let array as [Any]: try container.encode(array.map(AnyCodable.init(value:)))
    case let dict as [String: Any]: try container.encode(dict.mapValues(AnyCodable.init(value:)))
    default: try container.encodeNil()
    }
  }

  init(value: Any) { self.value = value }
}
