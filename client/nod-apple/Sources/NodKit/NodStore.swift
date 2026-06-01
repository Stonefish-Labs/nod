import Combine
import Foundation

public struct NodNotificationOpenRequest: Identifiable, Equatable, Sendable {
  public let id = UUID()
  public let eventId: String?
  public let channelId: String?
}

@MainActor
public final class NodStore: ObservableObject {
  @Published public var servers: [NodServerProfile] = []
  @Published public var selectedServerId: String?
  @Published public var baseURLString: String
  @Published public var deviceName: String
  @Published public var enrollmentCode: String = ""
  @Published public var currentUser: NodUser?
  @Published public var registeredDevices: [NodUserDevice] = []
  @Published public var channels: [NodChannel] = []
  @Published public var pendingCountsByChannel: [String: Int] = [:]
  @Published public var events: [NodEvent] = []
  @Published public var selectedChannelId: String?
  @Published public var selectedEventId: String?
  @Published public var notificationSound: String
  @Published public var lastError: String?
  @Published public var notificationPermissionIssue: String?
  @Published public var isRegistered: Bool = false
  @Published public var isSyncConnected: Bool = false
  @Published public internal(set) var notificationDeliveryMode: NodNotificationDeliveryMode = .push
  @Published public internal(set) var serverConnectionIssuesById: [String: String] = [:]
  @Published public var notificationOpenRequest: NodNotificationOpenRequest?

  public let platform: NodDevicePlatform
  public var presentLocalNotifications: Bool

  public var selectedServer: NodServerProfile? {
    guard let selectedServerId else {
      return servers.first
    }
    return servers.first { $0.id == selectedServerId } ?? servers.first
  }

  public var subscribedChannels: [NodChannel] {
    channels.filter(\.subscribed)
  }

  public var totalPendingCount: Int {
    pendingCountsByChannel.values.reduce(0, +)
  }

  public var alertMessage: String? {
    lastError ?? notificationPermissionIssue
  }

  public func connectionIssue(for server: NodServerProfile) -> String? {
    serverConnectionIssuesById[server.id]
  }

  public static let notificationSoundOptions: [NodNotificationSoundOption] = [
    NodNotificationSoundOption(id: "default", label: "Default"),
    NodNotificationSoundOption(id: "nod_ping.wav", label: "Ping"),
    NodNotificationSoundOption(id: "nod_chime.wav", label: "Chime"),
    NodNotificationSoundOption(id: "nod_low.wav", label: "Low"),
    NodNotificationSoundOption(id: "silent", label: "Silent"),
  ]

  static let applePushProvider = "apple_apns"

  let keychain = NodKeychainStore()
  let signingKeys = NodSigningKeyStore()
  let appAttest: NodAppAttestationProviding
  let defaults = UserDefaults.standard
  let sync = NodSyncClient()
  var pushToken: String?
  var tokenCache: [String: String] = [:]
  var loadedTokenServerIds = Set<String>()
  var knownPendingEventIds = Set<String>()
  var hasLoadedPendingEventSnapshot = false
  var syncReconnectTask: Task<Void, Never>?

  public init(
    platform: NodDevicePlatform,
    defaultDeviceName: String,
    presentLocalNotifications: Bool,
    appAttest: NodAppAttestationProviding = NodAppAttestationStore()
  ) {
    self.platform = platform
    self.presentLocalNotifications = presentLocalNotifications
    self.appAttest = appAttest

    let savedDeviceName = defaults.string(forKey: "nod.deviceName") ?? defaultDeviceName
    let savedBaseURL = NodServerAddress.normalizedBaseURL(
      defaults.string(forKey: "nod.baseURL") ?? ""
    )
    let savedDraftBaseURL = defaults.string(forKey: "nod.draft.baseURL").map(
      NodServerAddress.normalizedBaseURL
    )
    self.baseURLString = savedDraftBaseURL ?? savedBaseURL
    self.deviceName = savedDeviceName
    self.notificationSound = defaults.string(forKey: "nod.notificationSound") ?? "default"

    self.servers = Self.loadServers(from: defaults)
    let savedSelection = defaults.string(forKey: "nod.selectedServerId")
    self.selectedServerId =
      servers.contains { $0.id == savedSelection } ? savedSelection : servers.first?.id
    self.isRegistered = selectedServer != nil

    sync.onConnected = { [weak self] in
      Task { @MainActor in
        self?.isSyncConnected = true
        self?.markServerContactSucceeded()
      }
    }
    sync.onEnvelope = { [weak self] envelope in
      Task { @MainActor in
        await self?.handle(envelope: envelope)
      }
    }
    sync.onError = { [weak self] error in
      Task { @MainActor in
        self?.handleSyncError(error)
      }
    }

    NodNotificationController.shared.configure(
      apiProvider: { [weak self] in
        self?.api()
      },
      onOpen: { [weak self] eventId, channelId in
        Task { @MainActor in
          await self?.openNotification(eventId: eventId, channelId: channelId)
        }
      },
      onAction: { [weak self] eventId, actionId, text in
        await self?.submitNotificationAction(eventId: eventId, actionId: actionId, text: text)
      }
    )
  }

  public func api() -> NodAPI? {
    guard let server = selectedServer else {
      return nil
    }
    return api(for: server)
  }

  public func dismissAlertMessage() {
    if lastError != nil {
      lastError = nil
    } else {
      notificationPermissionIssue = nil
    }
  }

  public func selectServer(_ serverId: String) {
    guard selectedServerId != serverId else {
      return
    }
    selectedServerId = serverId
    defaults.set(serverId, forKey: "nod.selectedServerId")
    currentUser = nil
    registeredDevices = []
    channels = []
    pendingCountsByChannel = [:]
    events = []
    resetKnownPendingEvents()
    selectedChannelId = nil
    selectedEventId = nil
    sync.disconnect()
    isSyncConnected = false
    connectSync()
    Task { await refresh() }
  }
}
