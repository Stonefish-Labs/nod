import Foundation

public enum NodNotificationPolicy {
  /// WebSocket-only delivery means the app must surface local notifications itself.
  public static func shouldPresentLocalNotification(
    presentLocalNotifications: Bool,
    deliveryMode: NodNotificationDeliveryMode
  ) -> Bool {
    presentLocalNotifications || deliveryMode == .websocket
  }
}
