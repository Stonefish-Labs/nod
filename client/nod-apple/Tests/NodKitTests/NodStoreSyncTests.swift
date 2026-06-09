import Foundation
import XCTest

@testable import NodKit

/// After the cutover onto the shared Rust runtime, server/session lifecycle (the
/// API client, auth-error classification, server removal) lives in
/// `nod-client-core`. The Swift facade only still owns the re-enrollment UI flow:
/// prefilling the draft fields from the invalid server and, when other servers
/// remain, raising a registration prompt. Those are the behaviours covered here.
final class NodStoreSyncTests: XCTestCase {
  @MainActor
  func testInvalidSessionReEnrollmentPrefillsRegistrationDraft() {
    let store = testStore()
    store.servers = [serverProfile()]
    store.selectedServerId = "server-1"
    store.reEnrollmentServerId = "server-1"
    store.lastError = "invalid"

    store.beginInvalidSessionReEnrollment()

    XCTAssertEqual(store.baseURLString, "https://example.test/boop")
    XCTAssertEqual(store.deviceName, "Test Mac")
    XCTAssertEqual(store.enrollmentCode, "")
    XCTAssertNil(store.lastError)
    XCTAssertNil(store.reEnrollmentServerId)
    XCTAssertNil(store.registrationPromptRequestId)
  }

  @MainActor
  func testInvalidSessionReEnrollmentPromptsRegistrationWhenOtherServersRemain() {
    let store = testStore()
    store.servers = [
      serverProfile(),
      NodServerProfile(
        id: "server-2",
        name: "Other Server",
        baseURLString: "https://other.example.test",
        deviceName: "Other Mac",
        deviceId: "device-2"
      ),
    ]
    store.selectedServerId = "server-1"
    store.reEnrollmentServerId = "server-1"

    store.beginInvalidSessionReEnrollment()

    XCTAssertEqual(store.baseURLString, "https://example.test/boop")
    XCTAssertNotNil(store.registrationPromptRequestId)
  }

  @MainActor
  func testInvalidSessionReEnrollmentClearsWhenNoServerSelected() {
    let store = testStore()
    store.reEnrollmentServerId = "missing"

    store.beginInvalidSessionReEnrollment()

    XCTAssertNil(store.reEnrollmentServerId)
  }

  @MainActor
  private func testStore() -> NodStore {
    let store = NodStore(
      platform: .macos,
      defaultDeviceName: "Test Mac",
      presentLocalNotifications: false,
      configureNotificationController: false
    )
    store.servers = []
    store.selectedServerId = nil
    store.baseURLString = ""
    store.deviceName = "Test Mac"
    store.enrollmentCode = ""
    store.lastError = nil
    store.reEnrollmentServerId = nil
    store.registrationPromptRequestId = nil
    return store
  }

  private func serverProfile() -> NodServerProfile {
    NodServerProfile(
      id: "server-1",
      name: "Test Server",
      baseURLString: "https://example.test/boop",
      deviceName: "Test Mac",
      deviceId: "device-1"
    )
  }
}
