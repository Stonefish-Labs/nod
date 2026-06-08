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

  func testRequestInboxOrdersPendingBeforeHandled() throws {
    let pending = try makeRequest(id: "request-1", status: .pending, createdAt: "2026-05-31T10:00:00.000Z")
    let resolved = try makeRequest(id: "request-2", status: .resolved, createdAt: "2026-05-31T11:00:00.000Z")

    XCTAssertEqual(NodRequestInbox.pendingFirst([resolved, pending]).map(\.id), ["request-1", "request-2"])
  }

  func testRequestInboxCountsPendingBySource() throws {
    let requests = [
      try makeRequest(id: "request-1", sourceId: "deploy", status: .pending),
      try makeRequest(id: "request-2", sourceId: "deploy", status: .resolved),
      try makeRequest(id: "request-3", sourceId: "security", status: .pending),
    ]

    XCTAssertEqual(NodRequestInbox.pendingCountsBySource(in: requests), ["deploy": 1, "security": 1])
  }

  func testLegacyServerContractKeysDoNotDecode() throws {
    XCTAssertThrowsError(
      try JSONDecoder.nod.decode(SourcesResponse.self, from: #"{"channels":[]}"#.data(using: .utf8)!)
    )
    XCTAssertThrowsError(
      try JSONDecoder.nod.decode(RequestsResponse.self, from: #"{"events":[]}"#.data(using: .utf8)!)
    )
    XCTAssertThrowsError(
      try JSONDecoder.nod.decode(RequestResponse.self, from: #"{"event":{}}"#.data(using: .utf8)!)
    )
    XCTAssertThrowsError(
      try JSONDecoder.nod.decode(NodRequest.self, from: legacyRequestData(sourceKey: "channel_id", optionsKey: "options"))
    )
    XCTAssertThrowsError(
      try JSONDecoder.nod.decode(NodRequest.self, from: legacyRequestData(sourceKey: "source_id", optionsKey: "actions"))
    )
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

  private func makeRequest(
    id: String,
    sourceId: String = "source-1",
    status: NodRequestStatus,
    createdAt: String = "2026-05-31T12:00:00.000Z"
  ) throws -> NodRequest {
    let data = """
      {
        "id": "\(id)",
        "request_id": "\(id)",
        "source_id": "\(sourceId)",
        "recipients": [],
        "decision_resolution": "shared",
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
        "decision": null,
        "decisions": [],
        "callback_url": null,
        "request_digest": "digest-1",
        "options": []
      }
      """.data(using: .utf8)!
    return try JSONDecoder.nod.decode(NodRequest.self, from: data)
  }

  private func legacyRequestData(sourceKey: String, optionsKey: String) -> Data {
    """
      {
        "id": "request-legacy",
        "request_id": "request-legacy",
        "\(sourceKey)": "source-1",
        "recipients": [],
        "decision_resolution": "shared",
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
        "status": "pending",
        "created_at": "2026-05-31T12:00:00.000Z",
        "updated_at": "2026-05-31T12:00:00.000Z",
        "resolved_at": null,
        "decision": null,
        "decisions": [],
        "callback_url": null,
        "request_digest": "digest-1",
        "\(optionsKey)": []
      }
      """.data(using: .utf8)!
  }
}
