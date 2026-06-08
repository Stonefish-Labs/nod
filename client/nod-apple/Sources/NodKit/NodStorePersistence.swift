import Foundation

extension NodStore {
  static let clientSchemaVersion = 2

  func syncDevicePreferences() async {
    guard let api = api() else {
      return
    }
    do {
      try await api.updateDevicePreferences(notificationSound: notificationSound)
      markServerContactSucceeded()
      lastError = nil
    } catch {
      reportConnectionError(error)
    }
  }

  func api(for server: NodServerProfile) -> NodAPI? {
    guard let url = URL(string: server.baseURLString) else {
      return nil
    }
    let token = cachedToken(for: server)
    return NodAPI(baseURL: url, token: token ?? nil)
  }

  func cachedToken(for server: NodServerProfile) -> String? {
    if loadedTokenServerIds.contains(server.id) {
      return tokenCache[server.id]
    }

    loadedTokenServerIds.insert(server.id)
    let token = try? keychain.load(account: Self.tokenAccount(for: server.id))
    if let token {
      tokenCache[server.id] = token
    }
    return token
  }

  static func loadServers(from defaults: UserDefaults) -> [NodServerProfile] {
    guard let data = defaults.data(forKey: "nod.servers") else {
      return []
    }
    return (try? JSONDecoder().decode([NodServerProfile].self, from: data)) ?? []
  }

  static func saveServers(_ servers: [NodServerProfile], to defaults: UserDefaults) {
    guard let data = try? JSONEncoder().encode(servers) else {
      return
    }
    defaults.set(data, forKey: "nod.servers")
  }

  static func tokenAccount(for serverId: String) -> String {
    "serverToken.\(serverId)"
  }

  static func signingKeyAccount(for serverId: String) -> String {
    "decisionSigningKey.\(serverId)"
  }

  static func appAttestKeyAccount(for serverId: String) -> String {
    "appAttestKey.\(serverId)"
  }

  func resetLegacyClientStateIfNeeded() {
    let schemaKey = "nod.clientSchemaVersion"
    if defaults.integer(forKey: schemaKey) == Self.clientSchemaVersion,
      defaults.object(forKey: schemaKey) != nil
    {
      return
    }

    let preservedDeviceName = defaults.string(forKey: "nod.deviceName")
    let preservedNotificationSound = defaults.string(forKey: "nod.notificationSound")
    let oldServers = Self.loadServers(from: defaults)

    for server in oldServers {
      try? keychain.delete(account: Self.tokenAccount(for: server.id))
      try? signingKeys.delete(account: Self.signingKeyAccount(for: server.id))
      try? appAttest.delete(account: Self.appAttestKeyAccount(for: server.id))
    }

    [
      "nod.servers",
      "nod.selectedServerId",
      "nod.baseURL",
      "nod.draft.baseURL",
      "nod.deviceId",
    ].forEach(defaults.removeObject)

    if let preservedDeviceName {
      defaults.set(preservedDeviceName, forKey: "nod.deviceName")
    }
    if let preservedNotificationSound {
      defaults.set(preservedNotificationSound, forKey: "nod.notificationSound")
    }
    defaults.set(Self.clientSchemaVersion, forKey: schemaKey)
  }
}
