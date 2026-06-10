# Apple Client Setup

The package in `client/nod-apple` is intentionally SwiftUI-native UI over the
shared Rust client. The client logic (HTTP API, websocket sync, state, decision
orchestration) lives in `nod-client-core` and is driven through the
`NodClientFFI` UniFFI runtime; Swift implements only the native adapters. It
contains:

- `NodKit`: the runtime bridge (`NodRuntimeClient`/`NodRuntimeState`), the SwiftUI-facing `NodStore` facade, and the native adapters — Secure Enclave decision signing, App Attest, UserNotifications/APNs registration, markdown rendering.
- `Sources/NodClientFFI` + `Frameworks/`: the generated UniFFI wrapper and `nod_client_ffiFFI.xcframework` (git-ignored; rebuild with `scripts/build-nod-client-ffi.sh` whenever the Rust side changes).
- `Apps/NodMac`: macOS menu bar/window app shell.
- `Apps/NodIOS`: iOS app shell.
- `Nod.xcodeproj`: Xcode app targets for iOS and macOS, linked to the local `NodKit` package.
- A SwiftPM `NodMac` executable target that compiles the macOS app for quick local checks.

## Xcode

Full Xcode is installed at:

```bash
/Applications/Xcode.app
```

This machine currently selects Command Line Tools for command-line builds. When you are ready to build/sign the iOS app, switch Xcode selection:

```bash
sudo xcode-select -s /Applications/Xcode.app/Contents/Developer
```

Open the generated Xcode project:

```bash
open client/nod-apple/Nod.xcodeproj
```

The iOS target is configured with:

- Bundle ID: `com.batteryshark.Boop`
- Automatic signing
- `aps-environment = development` for Debug and `production` for Release/TestFlight
- `UIBackgroundModes = remote-notification`

The macOS target is configured as `com.batteryshark.NodMac`.

## Pairing and Servers

The Apple clients support multiple Nod servers. Pair each server with a short-lived pairing code, and the token is stored in Keychain under that server profile.

The apps do not ship with a default server URL. Enter the URL from your Nod server along with an enrollment code from the server's admin panel.

Pairing codes are entered through fixed uppercase boxes to avoid autocorrect and whitespace issues. After pairing, the app shows servers first, then subscribed channels, then the request list for the selected channel. Channel visibility is controlled from the Subscriptions sheet.

Build the local runnable macOS app bundle with the canonical script:

```bash
./scripts/build-macos-app
```

That script refreshes:

```bash
build/DerivedData/Build/Products/Release/Nod.app
```

Do not use `swift build --product NodMac` when you need the app bundle; it
only builds the SwiftPM executable at `.build/.../NodMac`.

Compile-check the shared client and macOS app channel without changing global Xcode selection:

```bash
DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer \
  CLANG_MODULE_CACHE_PATH=/private/tmp/nod-clang-cache \
  swift build --package-path client/nod-apple --scratch-path /private/tmp/nod-swiftpm --product NodMac
```

Compile-check the iOS Xcode app target without changing global Xcode selection:

```bash
DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer \
  xcodebuild -project client/nod-apple/Nod.xcodeproj -scheme NodIOS -configuration Debug -destination generic/platform=iOS -allowProvisioningUpdates build
```

## TestFlight

Apple requires an App Store Connect app record before uploaded builds can
appear in TestFlight. Create the app with bundle ID `com.batteryshark.Boop`,
then use a team App Store Connect API key to archive and upload from the
command line. A team key is preferred because the script allows Xcode to create
or update signing assets when automatic signing needs them.

Required local environment:

```bash
export APP_STORE_CONNECT_API_KEY_ID="..."
export APP_STORE_CONNECT_API_ISSUER_ID="..."
export APP_STORE_CONNECT_API_KEY_PATH="/path/to/AuthKey_....p8"
```

Then upload:

```bash
scripts/testflight-ios
```

By default, the script marks the upload as internal-TestFlight-only and uses a
UTC timestamp build number so each upload is accepted without editing project
files. To archive without uploading:

```bash
scripts/testflight-ios --archive-only
```

To allow the uploaded build to be distributed to external TestFlight testers
later:

```bash
scripts/testflight-ios --external
```

The script also accepts these optional overrides:

```bash
export NOD_MARKETING_VERSION="1.0"
export NOD_BUILD_NUMBER="202605271715"
export NOD_APPLE_TEAM_ID="Y734633UDM"
export NOD_IOS_BUNDLE_ID="com.batteryshark.Boop"
```

TestFlight builds use production APNs tokens, so the Nod server that receives
paired TestFlight devices should use the production Apple APNs provider:

```bash
NOD_APPLE_APNS_ENVIRONMENT=production
NOD_APPLE_APNS_BUNDLE_ID=com.batteryshark.Boop
```

When the Nod server reports `notification_delivery.mode = "websocket"`, the iOS
client presents WebSocket `created` requests as local notifications while the app
is active and connected. This is a foreground fallback only; background and
lock-screen delivery still require APNs. Direct APNs and notification relay
routes are transparent to Apple clients and both appear as `mode = "push"`.

## Notification Categories

The clients register:

- `NOD_DEFAULT`
- `NOD_APPROVAL`
- `NOD_APPROVAL_TEXT`

These categories map APNs notifications back to the server option endpoints.

## Notification Sounds

Notification sounds are a client preference, not a request field. Change the sound in the Apple client's Subscriptions sheet. The setting is synced to each paired server as a device preference because APNs requires the provider to include the sound filename in the per-device push payload.

Bundled options:

- Default
- Ping
- Chime
- Low
- Silent

iOS does not expose the built-in Messages/Text Tone sound catalog to third-party apps. Custom sounds need to be shipped in the app bundle or present in the app container's `Library/Sounds` directory, and must be shorter than 30 seconds.
