import AppKit
import NodKit
import SwiftUI

@main
struct NodMacApp: App {
    @NSApplicationDelegateAdaptor(AppDelegate.self) private var appDelegate
    @StateObject private var store = NodStore(
        platform: .macos,
        defaultDeviceName: Host.current().localizedName ?? "Mac",
        presentLocalNotifications: true
    )

    var body: some Scene {
        WindowGroup {
            NodRootView()
                .environmentObject(store)
                .frame(minWidth: 860, minHeight: 560)
        }
        .commands {
            CommandGroup(after: .appInfo) {
                Button("Refresh") {
                    Task { await store.refresh() }
                }
                .keyboardShortcut("r")
            }
        }

        MenuBarExtra {
            if store.totalPendingCount > 0 {
                Text("\(store.totalPendingCount) pending")
                Divider()
            }
            Button("Open Nod") {
                NSApp.activate(ignoringOtherApps: true)
                if NSApp.windows.isEmpty {
                    NSApp.sendAction(Selector(("showSettingsWindow:")), to: nil, from: nil)
                }
            }
            Button("Refresh") {
                Task { await store.refresh() }
            }
            Divider()
            Text("Notifications: \(notificationStatusLabel)")
            Button("Request Notification Permission") {
                Task { await store.requestNotifications() }
            }
            Button("Open Notification Settings") {
                openNotificationSettings()
            }
            Divider()
            Button("Quit") {
                NSApp.terminate(nil)
            }
        } label: {
            Label(menuBarTitle, systemImage: menuBarSystemImage)
                .task {
                    await store.refreshNotificationAuthorizationStatus()
                    updateDockBadge()
                }
                .onChange(of: store.totalPendingCount) {
                    updateDockBadge()
                }
        }
    }

    private var notificationStatusLabel: String {
        switch store.notificationAuthorizationStatus {
        case .authorized, .provisional, .ephemeral:
            return "Granted"
        case .denied:
            return "Denied"
        case .notDetermined:
            return "Not Determined"
        case .unknown:
            return "Unknown"
        }
    }

    private var menuBarTitle: String {
        guard store.totalPendingCount > 0 else {
            return "Nod"
        }
        return "Nod \(store.totalPendingCount)"
    }

    private var menuBarSystemImage: String {
        if store.totalPendingCount > 0 {
            return "bell.badge.fill"
        }
        return store.isSyncConnected ? "bell" : "bell.slash"
    }

    private func updateDockBadge() {
        NSApp.dockTile.badgeLabel = store.totalPendingCount > 0 ? "\(store.totalPendingCount)" : nil
    }

    private func openNotificationSettings() {
        let urls = [
            "x-apple.systempreferences:com.apple.preference.notifications?id=com.batteryshark.NodMac",
            "x-apple.systempreferences:com.apple.Notifications-Settings.extension"
        ]
        for urlString in urls {
            guard let url = URL(string: urlString), NSWorkspace.shared.open(url) else {
                continue
            }
            return
        }
    }
}

final class AppDelegate: NSObject, NSApplicationDelegate {
    func applicationDidFinishLaunching(_ notification: Notification) {
        NSApp.setActivationPolicy(.regular)
    }
}
