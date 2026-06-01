import NodKit
import SwiftUI
import UIKit

@main
struct NodIOSApp: App {
    @UIApplicationDelegateAdaptor(AppDelegate.self) private var appDelegate
    @Environment(\.scenePhase) private var scenePhase
    @StateObject private var store = NodStore(
        platform: .ios,
        defaultDeviceName: UIDevice.current.name,
        presentLocalNotifications: false
    )

    var body: some Scene {
        WindowGroup {
            NodRootView()
                .environmentObject(store)
            .task(id: scenePhase) {
                guard scenePhase == .active else {
                    return
                }
                await store.requestNotifications()
                await MainActor.run {
                    UIApplication.shared.registerForRemoteNotifications()
                }
            }
            .onReceive(NotificationCenter.default.publisher(for: .nodAPNSToken)) { notification in
                guard let token = notification.object as? String else {
                    return
                }
                Task {
                    await store.registerPushToken(token)
                }
            }
            .onReceive(NotificationCenter.default.publisher(for: .nodAPNSError)) { notification in
                if let error = notification.object as? Error {
                    store.lastError = error.localizedDescription
                }
            }
            .onReceive(NotificationCenter.default.publisher(for: .nodRemoteNotification)) { _ in
                Task { await store.refresh() }
            }
            .onChange(of: scenePhase) { _, phase in
                switch phase {
                case .active:
                    Task { await store.resumeFromForeground() }
                case .background:
                    store.disconnectSync()
                default:
                    break
                }
            }
        }
    }
}

final class AppDelegate: NSObject, UIApplicationDelegate {
    func application(
        _ application: UIApplication,
        didRegisterForRemoteNotificationsWithDeviceToken deviceToken: Data
    ) {
        let token = deviceToken.map { String(format: "%02x", $0) }.joined()
        NotificationCenter.default.post(name: .nodAPNSToken, object: token)
    }

    func application(
        _ application: UIApplication,
        didFailToRegisterForRemoteNotificationsWithError error: Error
    ) {
        NotificationCenter.default.post(name: .nodAPNSError, object: error)
    }

    func application(
        _ application: UIApplication,
        didReceiveRemoteNotification userInfo: [AnyHashable: Any],
        fetchCompletionHandler completionHandler: @escaping (UIBackgroundFetchResult) -> Void
    ) {
        NotificationCenter.default.post(name: .nodRemoteNotification, object: userInfo)
        completionHandler(.newData)
    }
}

extension Notification.Name {
    static let nodAPNSToken = Notification.Name("NodAPNSToken")
    static let nodAPNSError = Notification.Name("NodAPNSError")
    static let nodRemoteNotification = Notification.Name("NodRemoteNotification")
}
