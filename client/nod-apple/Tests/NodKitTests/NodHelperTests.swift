import Foundation
import XCTest

@testable import NodKit

final class NodHelperTests: XCTestCase {
  func testServerAddressNormalizationKeepsCurrentValuesOnly() {
    XCTAssertEqual(NodServerAddress.normalizedBaseURL("nod.example.test"), "https://nod.example.test")
    XCTAssertEqual(NodServerAddress.normalizedBaseURL("localhost:8765"), "http://localhost:8765")
    XCTAssertEqual(
      NodServerAddress.normalizedBaseURL("http://127.0.0.1:8767"),
      "http://127.0.0.1:8767"
    )
  }

  func testServerAddressProfileIdAndDisplayNameAreStable() {
    let url = "https://nod.example.test/team-a"

    XCTAssertEqual(NodServerAddress.profileId(for: url), "https-nod-example-test-team-a")
    XCTAssertEqual(NodServerAddress.displayName(for: url), "nod.example.test/team-a")
  }

  func testEventInboxOrdersPendingBeforeHandled() throws {
    let pending = try makeEvent(id: "event-1", status: .pending, createdAt: "2026-05-31T10:00:00.000Z")
    let resolved = try makeEvent(id: "event-2", status: .resolved, createdAt: "2026-05-31T11:00:00.000Z")

    XCTAssertEqual(NodEventInbox.pendingFirst([resolved, pending]).map(\.id), ["event-1", "event-2"])
  }

  func testEventInboxCountsPendingByChannel() throws {
    let events = [
      try makeEvent(id: "event-1", channelId: "deploy", status: .pending),
      try makeEvent(id: "event-2", channelId: "deploy", status: .resolved),
      try makeEvent(id: "event-3", channelId: "security", status: .pending),
    ]

    XCTAssertEqual(NodEventInbox.pendingCountsByChannel(in: events), ["deploy": 1, "security": 1])
  }

  func testMarkdownParserBuildsReadableBlocks() {
    let markdown = """
      # Deploy

      - Check metrics
      1. Approve
      > Keep an eye on latency.

      ```
      nod deploy
      ```
      ---
      """

    XCTAssertEqual(
      NodMarkdownParser.blocks(from: markdown),
      [
        .heading(level: 1, text: "Deploy"),
        .unorderedItem("Check metrics"),
        .orderedItem(marker: "1.", text: "Approve"),
        .quote("Keep an eye on latency."),
        .code("nod deploy"),
        .divider,
      ]
    )
  }

  private func makeEvent(
    id: String,
    channelId: String = "channel-1",
    status: NodEventStatus,
    createdAt: String = "2026-05-31T12:00:00.000Z"
  ) throws -> NodEvent {
    let data = """
      {
        "id": "\(id)",
        "channel_id": "\(channelId)",
        "recipients": [],
        "action_resolution": "shared",
        "title": "Deploy?",
        "summary": "Production deploy",
        "body_markdown": "Approve deploy",
        "fields": [],
        "links": [],
        "image_url": null,
        "priority": 1,
        "privacy": "normal",
        "dedupe_key": null,
        "expires_at": null,
        "status": "\(status.rawValue)",
        "created_at": "\(createdAt)",
        "updated_at": "\(createdAt)",
        "resolved_at": null,
        "result": null,
        "user_results": [],
        "callback_url": null,
        "request_digest": "digest-1",
        "actions": []
      }
      """.data(using: .utf8)!
    return try JSONDecoder.nod.decode(NodEvent.self, from: data)
  }
}
