import Foundation

/// Shared JSON coders for NodKit's wire models. The runtime (`nod-client-core`)
/// emits ISO-8601 timestamps with optional fractional seconds, so decoding the
/// `ClientState`/wire payloads needs the lenient date strategy below. These used
/// to live in `NodAPI.swift`; after the cutover onto the shared Rust runtime the
/// HTTP client is gone but the coders are still used by `NodRuntimeState` and the tests.
extension JSONDecoder {
  static var nod: JSONDecoder {
    let decoder = JSONDecoder()
    decoder.dateDecodingStrategy = .custom { decoder in
      let container = try decoder.singleValueContainer()
      let value = try container.decode(String.self)
      if let date = NodDateParser.date(from: value) {
        return date
      }
      throw DecodingError.dataCorruptedError(
        in: container, debugDescription: "Invalid ISO8601 date: \(value)")
    }
    return decoder
  }
}

extension JSONEncoder {
  static var nod: JSONEncoder {
    let encoder = JSONEncoder()
    encoder.dateEncodingStrategy = .iso8601
    return encoder
  }
}

enum NodDateParser {
  static func date(from value: String) -> Date? {
    if let date = iso8601Date(from: value, fractionalSeconds: true) {
      return date
    }
    if let normalized = normalizedMillisecondsTimestamp(value),
      let date = iso8601Date(from: normalized, fractionalSeconds: true)
    {
      return date
    }
    return iso8601Date(from: value, fractionalSeconds: false)
  }

  private static func iso8601Date(from value: String, fractionalSeconds: Bool) -> Date? {
    let formatter = ISO8601DateFormatter()
    if fractionalSeconds {
      formatter.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
    } else {
      formatter.formatOptions = [.withInternetDateTime]
    }
    return formatter.date(from: value)
  }

  private static func normalizedMillisecondsTimestamp(_ value: String) -> String? {
    guard let dot = value.firstIndex(of: ".") else {
      return nil
    }

    var cursor = value.index(after: dot)
    var digits = ""
    while cursor < value.endIndex, value[cursor].isNumber {
      digits.append(value[cursor])
      cursor = value.index(after: cursor)
    }

    guard !digits.isEmpty, cursor < value.endIndex else {
      return nil
    }

    let milliseconds = String(digits.prefix(3)).padding(toLength: 3, withPad: "0", startingAt: 0)
    return "\(value[..<dot]).\(milliseconds)\(value[cursor...])"
  }
}
