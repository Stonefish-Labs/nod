import Foundation
import XCTest

@testable import NodKit

final class NodStoreSyncTests: XCTestCase {
  func testAuthenticatedRequestErrorActionInvalidatesOnlyUnauthorizedResponses() {
    XCTAssertEqual(
      NodStore.authenticatedRequestErrorAction(for: NodAPIError.badStatus(401, "")),
      .serverRejectedSession
    )
  }

  func testAuthenticatedRequestErrorActionKeepsForbiddenSeparateFromInvalidSession() {
    XCTAssertEqual(
      NodStore.authenticatedRequestErrorAction(for: NodAPIError.badStatus(403, "")),
      .requestDenied
    )
  }

  func testAuthenticatedRequestErrorActionKeepsMissingLocalTokenSeparateFromInvalidSession() {
    XCTAssertEqual(
      NodStore.authenticatedRequestErrorAction(for: NodAPIError.missingToken),
      .missingLocalToken
    )
  }

  func testAuthenticatedRequestErrorActionTreatsOtherFailuresAsConnectionIssues() {
    XCTAssertEqual(
      NodStore.authenticatedRequestErrorAction(for: NodAPIError.badStatus(500, "")),
      .connectionIssue
    )
    XCTAssertEqual(
      NodStore.authenticatedRequestErrorAction(for: URLError(.timedOut)),
      .connectionIssue
    )
  }

  @MainActor
  func testInvalidSessionOffersReEnrollmentForSelectedServer() {
    let store = testStore()
    store.servers = [serverProfile()]
    store.selectedServerId = "server-1"
    store.isRegistered = true

    store.handleAuthenticatedRequestError(NodAPIError.badStatus(401, ""))

    XCTAssertEqual(store.reEnrollmentServerId, "server-1")
    XCTAssertTrue(store.canReEnrollInvalidSession)
    XCTAssertEqual(
      store.connectionIssue(for: store.servers[0]),
      "Your Nod session with Test Server is no longer valid. Re-enroll this device to continue."
    )
  }

  @MainActor
  func testInvalidSessionReEnrollmentPrefillsRegistrationAndRemovesInvalidServer() {
    let store = testStore()
    store.servers = [serverProfile()]
    store.selectedServerId = "server-1"
    store.isRegistered = true
    store.reEnrollmentServerId = "server-1"
    store.lastError = "invalid"

    store.beginInvalidSessionReEnrollment()

    XCTAssertEqual(store.baseURLString, "https://example.test/boop")
    XCTAssertEqual(store.deviceName, "Test Mac")
    XCTAssertEqual(store.enrollmentCode, "")
    XCTAssertNil(store.lastError)
    XCTAssertNil(store.reEnrollmentServerId)
    XCTAssertFalse(store.isRegistered)
    XCTAssertTrue(store.servers.isEmpty)
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
    store.isRegistered = true
    store.reEnrollmentServerId = "server-1"

    store.beginInvalidSessionReEnrollment()

    XCTAssertEqual(store.baseURLString, "https://example.test/boop")
    XCTAssertEqual(store.servers.map(\.id), ["server-2"])
    XCTAssertTrue(store.isRegistered)
    XCTAssertNotNil(store.registrationPromptRequestId)
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
    store.isRegistered = false
    store.baseURLString = ""
    store.deviceName = "Test Mac"
    store.enrollmentCode = ""
    store.lastError = nil
    store.reEnrollmentServerId = nil
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
