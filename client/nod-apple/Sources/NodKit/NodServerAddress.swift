import Foundation
import NodClientFFI

/// Server-address helpers. The implementations live in `nod-client-core` (Rust)
/// and are reached through `NodClientFFI`, so the Apple apps, the TUI, and the
/// desktop all share one canonical normalization/profile-id/display-name logic
/// instead of re-deriving it per platform. A parity test in `nod-client-ffi`
/// (`matches_nodkit_server_address_vectors`) pins these to the original Swift
/// outputs.
public enum NodServerAddress {
  public static func normalizedBaseURL(_ value: String) -> String {
    NodClientFFI.normalizeBaseUrl(value: value)
  }

  public static func profileId(for baseURLString: String) -> String {
    NodClientFFI.profileIdFor(baseUrl: baseURLString)
  }

  public static func displayName(for baseURLString: String) -> String {
    NodClientFFI.displayNameFor(baseUrl: baseURLString)
  }
}
