import Foundation
import XCTest

@testable import NodKit

final class NodAppAttestationTests: XCTestCase {
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
}
