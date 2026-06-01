import Foundation
import UserNotifications

public struct NodNotificationSettings: Sendable {
    public let authorizationStatus: NodNotificationAuthorizationStatus
    public let alertSetting: NodNotificationAlertSetting

    fileprivate init(_ settings: UNNotificationSettings) {
        self.authorizationStatus = NodNotificationAuthorizationStatus(settings.authorizationStatus)
        self.alertSetting = NodNotificationAlertSetting(settings.alertSetting)
    }
}

public enum NodNotificationAuthorizationStatus: Sendable {
    case notDetermined
    case denied
    case authorized
    case provisional
    case ephemeral
    case unknown

    fileprivate init(_ status: UNAuthorizationStatus) {
        switch status {
        case .notDetermined:
            self = .notDetermined
        case .denied:
            self = .denied
        case .authorized:
            self = .authorized
        case .provisional:
            self = .provisional
        case .ephemeral:
            self = .ephemeral
        @unknown default:
            self = .unknown
        }
    }
}

public enum NodNotificationAlertSetting: Sendable {
    case notSupported
    case disabled
    case enabled
    case unknown

    fileprivate init(_ setting: UNNotificationSetting) {
        switch setting {
        case .notSupported:
            self = .notSupported
        case .disabled:
            self = .disabled
        case .enabled:
            self = .enabled
        @unknown default:
            self = .unknown
        }
    }
}

@MainActor
public final class NodNotificationController: NSObject, UNUserNotificationCenterDelegate {
    public static let shared = NodNotificationController()

    private var apiProvider: (() -> NodAPI?)?
    private var openHandler: (@MainActor (_ eventId: String?, _ channelId: String?) -> Void)?
    private var actionHandler: (@Sendable (_ eventId: String, _ actionId: String, _ text: String?) async -> Void)?

    public func configure(
        apiProvider: @escaping () -> NodAPI?,
        onOpen: @escaping @MainActor (_ eventId: String?, _ channelId: String?) -> Void = { _, _ in },
        onAction: @escaping @Sendable (_ eventId: String, _ actionId: String, _ text: String?) async -> Void = { _, _, _ in }
    ) {
        self.apiProvider = apiProvider
        self.openHandler = onOpen
        self.actionHandler = onAction
        let center = UNUserNotificationCenter.current()
        center.delegate = self
        registerCategories()
    }

    public func requestAuthorization() async throws -> Bool {
        try await UNUserNotificationCenter.current().requestAuthorization(
            options: [.alert, .badge, .sound, .providesAppNotificationSettings]
        )
    }

    public func notificationSettings() async -> NodNotificationSettings {
        await withCheckedContinuation { continuation in
            UNUserNotificationCenter.current().getNotificationSettings { settings in
                continuation.resume(returning: NodNotificationSettings(settings))
            }
        }
    }

    public func registerCategories() {
        let approve = UNNotificationAction(identifier: "approve", title: "Approve")
        let reject = UNNotificationAction(identifier: "reject", title: "Reject", options: [.destructive])
        let approveNotes = UNTextInputNotificationAction(
            identifier: "approve_notes",
            title: "Approve with Notes",
            options: [],
            textInputButtonTitle: "Approve",
            textInputPlaceholder: "Notes"
        )
        let rejectReason = UNTextInputNotificationAction(
            identifier: "reject_reason",
            title: "Reject with Reason",
            options: [.destructive],
            textInputButtonTitle: "Reject",
            textInputPlaceholder: "Reason"
        )
        let open = UNNotificationAction(identifier: "open", title: "Open", options: [.foreground])

        let defaultCategory = UNNotificationCategory(
            identifier: "NOD_DEFAULT",
            actions: [open],
            intentIdentifiers: []
        )
        let approvalCategory = UNNotificationCategory(
            identifier: "NOD_APPROVAL",
            actions: [approve, reject, open],
            intentIdentifiers: []
        )
        let approvalTextCategory = UNNotificationCategory(
            identifier: "NOD_APPROVAL_TEXT",
            actions: [approve, approveNotes, reject, rejectReason, open],
            intentIdentifiers: []
        )
        UNUserNotificationCenter.current().setNotificationCategories([
            defaultCategory,
            approvalCategory,
            approvalTextCategory
        ])
    }

    nonisolated public func presentLocalNotification(for event: NodEvent, soundName: String) async throws {
        let content = UNMutableNotificationContent()
        content.title = event.title
        content.body = event.summary.isEmpty ? event.bodyMarkdown : event.summary
        content.sound = notificationSound(named: soundName)
        content.threadIdentifier = event.channelId
        content.categoryIdentifier = category(for: event)
        content.userInfo = [
            "event_id": event.id,
            "channel_id": event.channelId
        ]
        if let attachment = try? await imageAttachment(for: event) {
            content.attachments = [attachment]
        }
        let request = UNNotificationRequest(identifier: event.id, content: content, trigger: nil)
        try await UNUserNotificationCenter.current().add(request)
    }

    nonisolated public func presentTestNotification(soundName: String) async throws {
        let content = UNMutableNotificationContent()
        content.title = "Nod notifications are on"
        content.body = "Desktop alerts will appear for new Nod items."
        content.sound = notificationSound(named: soundName)
        let request = UNNotificationRequest(
            identifier: "nod.notification-test.\(UUID().uuidString)",
            content: content,
            trigger: nil
        )
        try await UNUserNotificationCenter.current().add(request)
    }

    nonisolated public func userNotificationCenter(
        _ center: UNUserNotificationCenter,
        willPresent notification: UNNotification
    ) async -> UNNotificationPresentationOptions {
        [.banner, .sound, .list]
    }

    nonisolated public func userNotificationCenter(
        _ center: UNUserNotificationCenter,
        didReceive response: UNNotificationResponse
    ) async {
        let userInfo = response.notification.request.content.userInfo
        let eventId = Self.notificationString("event_id", in: userInfo)
        let channelId = Self.notificationString("channel_id", in: userInfo)

        if response.actionIdentifier == UNNotificationDefaultActionIdentifier || response.actionIdentifier == "open" {
            await MainActor.run {
                self.openHandler?(eventId, channelId)
            }
            return
        }

        if response.actionIdentifier == UNNotificationDismissActionIdentifier {
            return
        }

        guard
            let eventId
        else {
            return
        }
        let actionIdentifier = response.actionIdentifier
        let text = (response as? UNTextInputNotificationResponse)?.userText
        let handler = await MainActor.run { self.actionHandler }
        await handler?(eventId, actionIdentifier, text)
    }

    nonisolated private static func notificationString(_ key: String, in userInfo: [AnyHashable: Any]) -> String? {
        // Local notifications use flat keys, while server push payloads may nest
        // Nod metadata under "nod"; both shapes should open the same event.
        if let value = userInfo[key] as? String {
            return value
        }
        if let nod = userInfo["nod"] as? [String: Any], let value = nod[key] as? String {
            return value
        }
        if let nod = userInfo["nod"] as? [AnyHashable: Any], let value = nod[key] as? String {
            return value
        }
        return nil
    }

    nonisolated private func category(for event: NodEvent) -> String {
        if event.actions.contains(where: { $0.requiresText }) {
            return "NOD_APPROVAL_TEXT"
        }
        if event.actions.isEmpty {
            return "NOD_DEFAULT"
        }
        return "NOD_APPROVAL"
    }

    nonisolated private func imageAttachment(for event: NodEvent) async throws -> UNNotificationAttachment? {
        guard
            let imageUrl = event.imageUrl?.trimmingCharacters(in: .whitespacesAndNewlines),
            !imageUrl.isEmpty,
            let url = URL(string: imageUrl),
            let scheme = url.scheme?.lowercased(),
            scheme == "https" || scheme == "http"
        else {
            return nil
        }

        let (downloadedURL, response) = try await URLSession.shared.download(from: url)
        guard let fileExtension = Self.attachmentFileExtension(for: url, response: response) else {
            return nil
        }

        // UNNotificationAttachment needs a readable local file with a supported
        // filename suffix; URLSession's temporary download URL often has neither.
        let directory = FileManager.default
            .temporaryDirectory
            .appendingPathComponent("NodNotificationAttachments", isDirectory: true)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)

        let fileURL = directory
            .appendingPathComponent(Self.safeAttachmentFilename(event.id))
            .appendingPathExtension(fileExtension)
        try? FileManager.default.removeItem(at: fileURL)
        try FileManager.default.copyItem(at: downloadedURL, to: fileURL)

        return try UNNotificationAttachment(
            identifier: "image-\(event.id)",
            url: fileURL,
            options: nil
        )
    }

    nonisolated private static func attachmentFileExtension(for url: URL, response: URLResponse) -> String? {
        let pathExtension = url.pathExtension.lowercased()
        let supportedExtensions = Set(["jpg", "jpeg", "png", "gif", "heic", "heif", "tif", "tiff", "bmp"])
        if supportedExtensions.contains(pathExtension) {
            return pathExtension
        }

        switch response.mimeType?.lowercased() {
        case "image/jpeg":
            return "jpg"
        case "image/png":
            return "png"
        case "image/gif":
            return "gif"
        case "image/heic":
            return "heic"
        case "image/heif":
            return "heif"
        case "image/tiff":
            return "tiff"
        case "image/bmp":
            return "bmp"
        default:
            return nil
        }
    }

    nonisolated private static func safeAttachmentFilename(_ value: String) -> String {
        let scalars = value.unicodeScalars.map { scalar in
            CharacterSet.alphanumerics.contains(scalar) ? Character(scalar) : "-"
        }
        let filename = String(scalars).trimmingCharacters(in: CharacterSet(charactersIn: "-"))
        return filename.isEmpty ? UUID().uuidString : String(filename.prefix(80))
    }

    nonisolated private func notificationSound(named sound: String?) -> UNNotificationSound? {
        guard let sound = sound?.trimmingCharacters(in: .whitespacesAndNewlines), !sound.isEmpty else {
            return .default
        }
        if sound == "none" || sound == "silent" {
            return nil
        }
        if sound == "default" {
            return .default
        }
        return UNNotificationSound(named: UNNotificationSoundName(sound))
    }
}
