import Foundation

extension NodStore {
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
}
