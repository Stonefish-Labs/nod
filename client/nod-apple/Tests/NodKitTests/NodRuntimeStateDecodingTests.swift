import Foundation
import XCTest

@testable import NodKit

/// Decodes a fully-populated `ClientState` envelope — the shape nod-client-core
/// actually emits, with ISO-8601 dates, a device carrying `has_signing_key` +
/// `attestation`, and a request with options. Proves NodKit's reused wire models
/// decode real runtime state, which is the prerequisite for pointing the views
/// at `NodRuntimeClient` and deleting the duplicated Swift client.
final class NodRuntimeStateDecodingTests: XCTestCase {
  func testDecodesPopulatedStateEnvelope() throws {
    let json = """
      {
        "kind": "state",
        "payload": {
          "servers": [
            {
              "id": "https-nod-example-test",
              "name": "nod.example.test",
              "base_url_string": "https://nod.example.test",
              "device_name": "Studio",
              "device_id": "device-1",
              "user_id": "user-1",
              "user_name": "Mercaldi"
            }
          ],
          "selected_server_id": "https-nod-example-test",
          "current_user": {
            "id": "user-1",
            "name": "Mercaldi",
            "created_at": "2026-05-31T12:00:00.000Z",
            "updated_at": "2026-05-31T12:00:00.000Z"
          },
          "devices": [
            {
              "id": "device-1",
              "user_id": "user-1",
              "name": "Studio",
              "platform": "macos",
              "native_app_id": "com.stonefishlabs.nod",
              "push_provider": "apple_apns",
              "has_push_token": true,
              "has_signing_key": true,
              "attestation": {
                "provider": "app_attest",
                "status": "verified",
                "key_id": "attest-key-1",
                "team_id": "TEAM123",
                "bundle_id": "com.stonefishlabs.nod",
                "environment": "production",
                "verified_at": "2026-05-31T12:00:00.000Z",
                "failure_reason": null
              },
              "notification_sound": "default",
              "last_seen_at": "2026-05-31T12:00:00.000Z",
              "created_at": "2026-05-31T12:00:00.000Z",
              "is_current": true
            }
          ],
          "channels": [
            {
              "id": "deployments",
              "name": "Deployments",
              "emoji": "🚀",
              "subscribed": true,
              "created_at": "2026-05-31T12:00:00.000Z"
            }
          ],
          "pending_counts_by_channel": { "deployments": 1, "alerts": 0 },
          "requests": [
            {
              "id": "request-1",
              "request_id": "request-1",
              "channel_id": "deployments",
              "recipients": [],
              "decision_resolution": "shared",
              "title": "Deploy?",
              "summary": "Production deploy",
              "body_markdown": "Approve deploy",
              "fields": [],
              "links": [],
              "image_url": null,
              "notification": { "redact": false, "title": null, "body": null },
              "dedupe_key": null,
              "expires_at": null,
              "status": "pending",
              "created_at": "2026-05-31T12:00:00.000Z",
              "updated_at": "2026-05-31T12:00:00.000Z",
              "resolved_at": null,
              "decision": null,
              "decisions": [],
              "callback_url": null,
              "request_digest": "digest-1",
              "options": [
                {
                  "id": "approve",
                  "label": "Approve",
                  "kind": "approve",
                  "style": "default",
                  "requires_text": false,
                  "text_placeholder": null,
                  "destructive": false,
                  "foreground": false
                }
              ]
            }
          ],
          "selected_channel_id": "deployments",
          "selected_request_id": "request-1",
          "notification_sound": "nod_ping.wav",
          "notification_delivery_mode": "push",
          "is_registered": true,
          "is_sync_connected": true,
          "last_error": null
        }
      }
      """

    let message = try NodRuntimeMessage(from: Data(json.utf8))
    guard case .state(let state) = message else {
      return XCTFail("expected a state message")
    }

    XCTAssertTrue(state.isRegistered)
    XCTAssertTrue(state.isSyncConnected)
    XCTAssertEqual(state.notificationDeliveryMode, .push)
    // A count of 1 (and 0) is the regression trigger: the old AnyCodable
    // round-trip decoded JSON 1/0 as Bool, corrupting the [String:Int] map.
    XCTAssertEqual(state.pendingCountsByChannel["deployments"], 1)
    XCTAssertEqual(state.pendingCountsByChannel["alerts"], 0)
    XCTAssertEqual(state.totalPendingCount, 1)

    let server = try XCTUnwrap(state.selectedServer)
    XCTAssertEqual(server.baseUrlString, "https://nod.example.test")
    XCTAssertEqual(server.userName, "Mercaldi")

    XCTAssertEqual(state.currentUser?.name, "Mercaldi")

    let device = try XCTUnwrap(state.devices.first)
    XCTAssertTrue(device.hasSigningKey)
    XCTAssertEqual(device.platform, .macos)
    XCTAssertEqual(device.attestation?.status, .verified)
    XCTAssertEqual(device.attestation?.keyId, "attest-key-1")

    let channel = try XCTUnwrap(state.subscribedChannels.first)
    XCTAssertEqual(channel.emoji, "🚀")

    let request = try XCTUnwrap(state.requests.first)
    XCTAssertEqual(request.requestDigest, "digest-1")
    XCTAssertEqual(request.options.first?.kind, .approve)
  }
}
