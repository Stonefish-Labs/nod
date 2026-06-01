import Foundation
import NodKit
import SwiftUI

#if os(macOS)
import AppKit
#endif

struct StatusBadge: View {
  let status: NodEventStatus

  var body: some View {
    Text(status.rawValue.capitalized)
      .font(.caption)
      .fontWeight(.medium)
      .padding(.horizontal, 8)
      .padding(.vertical, 4)
      .background(color.opacity(0.16), in: Capsule())
      .foregroundStyle(color)
  }

  private var color: Color {
    switch status {
    case .pending:
      return .blue
    case .resolved:
      return .green
    case .expired:
      return .orange
    case .cancelled:
      return .secondary
    }
  }
}

#if os(macOS)
func openNodNotificationSettings() {
  let urls = [
    "x-apple.systempreferences:com.apple.preference.notifications?id=com.batteryshark.NodMac",
    "x-apple.systempreferences:com.apple.Notifications-Settings.extension",
  ]
  for urlString in urls {
    guard let url = URL(string: urlString), NSWorkspace.shared.open(url) else {
      continue
    }
    return
  }
}
#endif

struct AppBuildLabel: View {
  var body: some View {
    Text(AppBuildInfo.displayText)
      .font(.caption)
      .monospacedDigit()
      .foregroundStyle(.secondary)
      .textSelection(.enabled)
      .accessibilityLabel("App version \(AppBuildInfo.accessibilityText)")
  }
}

enum AppBuildInfo {
  static var displayText: String {
    "Version \(version) (\(build))"
  }

  static var accessibilityText: String {
    "\(version), build \(build)"
  }

  private static var version: String {
    bundleValue("CFBundleShortVersionString", fallback: "1.0")
  }

  private static var build: String {
    bundleValue("CFBundleVersion", fallback: "dev")
  }

  private static func bundleValue(_ key: String, fallback: String) -> String {
    Bundle.main.object(forInfoDictionaryKey: key) as? String ?? fallback
  }
}

func iconName(_ icon: String) -> String {
  switch icon {
  case "agent":
    return "cpu"
  case "deploy":
    return "shippingbox"
  case "security":
    return "lock.shield"
  default:
    return icon.isEmpty ? "bell" : icon
  }
}
