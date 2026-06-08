import Foundation
import XCTest

@testable import NodKit

final class NodDateDecodingTests: XCTestCase {
  func testDecoderAcceptsServerTimestampPrecisions() throws {
    let timestamps = [
      "2026-05-28T12:34:56Z",
      "2026-05-28T12:34:56.123Z",
      "2026-05-28T12:34:56.123456789Z",
      "2026-05-28T12:34:56.123456789+00:00",
    ]

    for timestamp in timestamps {
      let data = #"{"date":"\#(timestamp)"}"#.data(using: .utf8)!
      XCTAssertNoThrow(try JSONDecoder.nod.decode(DatePayload.self, from: data), timestamp)
    }
  }

  func testUnknownOptionKindDecodesAsCustom() throws {
    let known = try JSONDecoder().decode(
      NodOptionKind.self, from: #""reject_with_text""#.data(using: .utf8)!)
    let unknown = try JSONDecoder().decode(
      NodOptionKind.self, from: #""reject_reason""#.data(using: .utf8)!)

    XCTAssertEqual(known, .rejectWithText)
    XCTAssertEqual(unknown, .custom)
  }

  func testNotificationDeliveryDecodesFromSyncHello() throws {
    let data = """
      {
        "kind": "hello",
        "at": "2026-05-31T12:00:00.000Z",
        "payload": {
          "device_id": "device-1",
          "notification_delivery": { "mode": "websocket" }
        }
      }
      """.data(using: .utf8)!

    let envelope = try JSONDecoder.nod.decode(NodSyncEnvelope.self, from: data)

    XCTAssertEqual(envelope.kind, "hello")
    XCTAssertEqual(envelope.notificationDelivery?.mode, .websocket)
  }

  func testLocalNotificationPresentationFollowsDeliveryMode() {
    XCTAssertFalse(
      NodNotificationPolicy.shouldPresentLocalNotification(
        presentLocalNotifications: false,
        deliveryMode: .push
      )
    )

    XCTAssertTrue(
      NodNotificationPolicy.shouldPresentLocalNotification(
        presentLocalNotifications: false,
        deliveryMode: .websocket
      )
    )

    XCTAssertTrue(
      NodNotificationPolicy.shouldPresentLocalNotification(
        presentLocalNotifications: true,
        deliveryMode: .push
      )
    )
  }
}

private struct DatePayload: Decodable {
  let date: Date
}
