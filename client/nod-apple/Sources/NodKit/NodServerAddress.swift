import Foundation

public enum NodServerAddress {
  public static func normalizedBaseURL(_ value: String) -> String {
    let trimmed = value.trimmingCharacters(in: .whitespacesAndNewlines)
      .trimmingCharacters(in: CharacterSet(charactersIn: "/"))
    guard !trimmed.isEmpty else {
      return ""
    }
    if trimmed.contains("://") {
      return trimmed
    }
    // Localhost and 192.168.* servers are expected to be development boxes
    // without TLS; public hostnames default to HTTPS.
    if trimmed == "localhost" || trimmed.hasPrefix("localhost:") || trimmed.hasPrefix("127.")
      || trimmed.hasPrefix("192.168.")
    {
      return "http://\(trimmed)"
    }
    return "https://\(trimmed)"
  }

  public static func profileId(for baseURLString: String) -> String {
    let mapped = baseURLString.lowercased().unicodeScalars.map { scalar in
      CharacterSet.alphanumerics.contains(scalar) ? String(scalar) : "-"
    }.joined()
    let compact = mapped.split(separator: "-").joined(separator: "-")
    return compact.isEmpty ? UUID().uuidString.lowercased() : String(compact.prefix(80))
  }

  public static func displayName(for baseURLString: String) -> String {
    guard let url = URL(string: baseURLString) else {
      return "Nod Server"
    }
    var name = url.host ?? "Nod Server"
    let path = url.path.trimmingCharacters(in: CharacterSet(charactersIn: "/"))
    if !path.isEmpty {
      name += "/\(path)"
    }
    return name
  }
}
