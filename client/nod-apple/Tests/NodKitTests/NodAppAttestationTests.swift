import Foundation
import XCTest

@testable import NodKit

final class NodAppAttestationTests: XCTestCase {
  override func tearDown() {
    TestURLProtocol.requestHandler = nil
    TestURLProtocol.lastBody = nil
    super.tearDown()
  }

  func testEnrollmentClientDataMatchesServerVector() {
    let signingKey = NodDeviceSigningKey(
      keyId: "device-key-id",
      algorithm: "p256_ecdsa_sha256",
      publicKey: "base64url-public-key"
    )

    let clientData = NodEnrollmentAttestation.clientData(
      code: " ABCD1234 ",
      deviceName: " iPhone ",
      platform: .ios,
      pushProvider: " apple_apns ",
      pushToken: " provider-token ",
      signingKey: signingKey,
      appAttestKeyId: "app-attest-key"
    )

    XCTAssertEqual(
      clientData,
      """
      nod-enrollment-v1
      code_sha256:1635c8525afbae58c37bede3c9440844e9143727cc7c160bed665ec378d8a262
      device_name_sha256:38fdf519314e3151d7e7f6ef456f327b78ddb84bc457bdb0d49bce0b1fc3c959
      platform:ios
      push_provider:apple_apns
      push_token_sha256:2ad21144ec11edbd553556e1dcd9a79383adbf4ae0e14266a19977edc3de9257
      signing_key_id:device-key-id
      signing_key_algorithm:p256_ecdsa_sha256
      signing_public_key_sha256:b848efd33d347196ccd5140f15975ed8d1f91cb2e99e2571fa5bd09282d5cc6f
      attestation_provider:apple_app_attest
      attestation_key_id:app-attest-key

      """
    )
  }

  func testEnrollIncludesProvidedAttestation() async throws {
    let api = makeAPI()
    let signingKey = NodDeviceSigningKey(
      keyId: "device-key-id",
      algorithm: "p256_ecdsa_sha256",
      publicKey: "base64url-public-key"
    )
    let attestation = NodDeviceAttestation(
      provider: "apple_app_attest",
      keyId: "app-attest-key",
      attestationObject: "base64url-cbor"
    )

    _ = try await api.enroll(NodEnrollmentRequest(
      code: "ABCDEFGH",
      deviceName: "iPhone",
      platform: .ios,
      nativeAppId: "com.example.NodTests",
      signingKey: signingKey,
      attestation: attestation
    ))

    let body = try requestBody()
    XCTAssertEqual(body["native_app_id"] as? String, "com.example.NodTests")
    let attestationBody = try XCTUnwrap(body["attestation"] as? [String: Any])
    XCTAssertEqual(attestationBody["provider"] as? String, "apple_app_attest")
    XCTAssertEqual(attestationBody["key_id"] as? String, "app-attest-key")
    XCTAssertEqual(attestationBody["attestation_object"] as? String, "base64url-cbor")
  }

  func testEnrollOmitsUnsupportedAttestation() async throws {
    let api = makeAPI()
    let signingKey = NodDeviceSigningKey(
      keyId: "device-key-id",
      algorithm: "p256_ecdsa_sha256",
      publicKey: "base64url-public-key"
    )

    _ = try await api.enroll(NodEnrollmentRequest(
      code: "ABCDEFGH",
      deviceName: "iPhone",
      platform: .ios,
      signingKey: signingKey,
      attestation: nil
    ))

    let body = try requestBody()
    XCTAssertNil(body["attestation"])
  }

  func testUpdatePushTokenIncludesNativeAppId() async throws {
    let api = makeAPI(responseData: Data())
    api.token = "nod_device_test"

    try await api.updatePushToken(
      provider: "apple_apns",
      nativeAppId: "com.example.NodTests",
      token: "apns-token"
    )

    let body = try requestBody()
    XCTAssertEqual(body["provider"] as? String, "apple_apns")
    XCTAssertEqual(body["native_app_id"] as? String, "com.example.NodTests")
    XCTAssertEqual(body["token"] as? String, "apns-token")
  }

  private func makeAPI(responseData: Data? = nil) -> NodAPI {
    let responseData = responseData ?? Self.enrollResponseData()
    TestURLProtocol.requestHandler = { request in
      TestURLProtocol.lastBody = request.httpBody ?? request.httpBodyStream.flatMap(Data.reading)
      let response = HTTPURLResponse(
        url: request.url!,
        statusCode: 200,
        httpVersion: nil,
        headerFields: ["Content-Type": "application/json"]
      )!
      return (response, responseData)
    }
    let configuration = URLSessionConfiguration.ephemeral
    configuration.protocolClasses = [TestURLProtocol.self]
    return NodAPI(
      baseURL: URL(string: "https://nod.example.test")!,
      session: URLSession(configuration: configuration)
    )
  }

  private func requestBody() throws -> [String: Any] {
    let bodyData = try XCTUnwrap(TestURLProtocol.lastBody)
    let value = try JSONSerialization.jsonObject(with: bodyData)
    return try XCTUnwrap(value as? [String: Any])
  }

  private static func enrollResponseData() -> Data {
    """
    {
      "device_id": "device-1",
      "user_id": "owner",
      "user_name": "Owner",
      "token": "nod_device_test",
      "notification_delivery": { "mode": "websocket" },
      "channels": [],
      "devices": []
    }
    """.data(using: .utf8)!
  }
}

private final class TestURLProtocol: URLProtocol {
  nonisolated(unsafe) static var requestHandler: ((URLRequest) throws -> (HTTPURLResponse, Data))?
  nonisolated(unsafe) static var lastBody: Data?

  override class func canInit(with request: URLRequest) -> Bool {
    true
  }

  override class func canonicalRequest(for request: URLRequest) -> URLRequest {
    request
  }

  override func startLoading() {
    guard let handler = Self.requestHandler else {
      client?.urlProtocol(self, didFailWithError: URLError(.badServerResponse))
      return
    }
    do {
      let (response, data) = try handler(request)
      client?.urlProtocol(self, didReceive: response, cacheStoragePolicy: .notAllowed)
      client?.urlProtocol(self, didLoad: data)
      client?.urlProtocolDidFinishLoading(self)
    } catch {
      client?.urlProtocol(self, didFailWithError: error)
    }
  }

  override func stopLoading() {}
}

private extension Data {
  static func reading(_ stream: InputStream) -> Data? {
    stream.open()
    defer { stream.close() }

    var data = Data()
    var buffer = [UInt8](repeating: 0, count: 4096)
    while stream.hasBytesAvailable {
      let count = stream.read(&buffer, maxLength: buffer.count)
      if count < 0 {
        return nil
      }
      if count == 0 {
        break
      }
      data.append(buffer, count: count)
    }
    return data
  }
}
