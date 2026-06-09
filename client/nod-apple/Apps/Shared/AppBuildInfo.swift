import Foundation
import NodKit
import SwiftUI

#if os(macOS)
import AppKit
#endif

struct StatusBadge: View {
  private let presentation: StatusBadgePresentation

  init(status: NodRequestStatus) {
    self.presentation = .status(status)
  }

  init(request: NodRequest) {
    self.presentation = .request(request)
  }

  var body: some View {
    Text(presentation.label)
      .font(.caption)
      .fontWeight(.medium)
      .padding(.horizontal, 8)
      .padding(.vertical, 4)
      .background(presentation.color.opacity(0.16), in: Capsule())
      .foregroundStyle(presentation.color)
  }
}

private struct StatusBadgePresentation {
  let label: String
  let color: Color

  static func request(_ request: NodRequest) -> Self {
    guard request.status == .resolved, let decision = request.decision else {
      return status(request.status)
    }
    return decisionOutcome(decision)
  }

  static func status(_ status: NodRequestStatus) -> Self {
    switch status {
    case .pending:
      return Self(label: "Pending", color: .blue)
    case .resolved:
      return Self(label: "Resolved", color: .green)
    case .expired:
      return Self(label: "Expired", color: .orange)
    case .cancelled:
      return Self(label: "Cancelled", color: .secondary)
    }
  }

  private static func decisionOutcome(_ decision: NodDecision) -> Self {
    switch decision.optionKind {
    case .approve, .approveWithText:
      return Self(label: "Approved", color: .green)
    case .reject, .rejectWithText:
      return Self(label: "Rejected", color: .red)
    case .dismiss:
      return Self(label: "Dismissed", color: .secondary)
    case .open:
      return Self(label: "Opened", color: .blue)
    case .custom:
      let label = decision.optionLabel.trimmingCharacters(in: .whitespacesAndNewlines)
      return Self(label: label.isEmpty ? "Resolved" : label, color: .secondary)
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
