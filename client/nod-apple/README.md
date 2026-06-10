<p align="center">
  <img src="../../assets/nod-icon.png" width="180" alt="Nod">
</p>

# nod-apple

Native SwiftUI macOS and iOS clients for [nod-server](../../server/nod-server).

One Swift package holds the shared `NodKit` library and both app shells. ~95% of the code is shared between platforms; only the `@main` entry points, entitlements, and `Info.plist` are platform-specific.

## Layout

The apps are a thin native shell over the shared Rust client
([nod-client-core](../nod-client-core)), reached through the single UniFFI
runtime ([nod-client-ffi](../../nod-client-ffi)). Swift keeps only what must be
native: Secure Enclave signing, App Attest, UserNotifications/APNs, and SwiftUI.

```
Package.swift           NodKit library + SwiftPM NodMac executable (fast iteration)
Sources/NodKit/         Runtime bridge (NodRuntimeClient/NodRuntimeState), the
                        SwiftUI-facing NodStore facade, and the native adapters:
                        Secure Enclave signer, App Attest, notifications, keychain
Sources/NodClientFFI/   Generated UniFFI Swift wrapper (git-ignored, built from source)
Frameworks/             nod_client_ffiFFI.xcframework (built from source, git-ignored)
Apps/
  NodMac/              macOS @main, entitlements, Info.plist, resources
  NodIOS/              iOS @main, entitlements, Info.plist, resources
  Shared/               SwiftUI views, sounds, asset catalog
Nod.xcodeproj/         Xcode targets for both apps
scripts/                FFI/app build, release & compliance scripts
```

## Build

The client logic (API, sync, state, decision orchestration) and the canonical
decision-signing bytes live once in Rust; Swift drives them through the
`NodClientFFI` module and signs in the Secure Enclave via a callback. The
`nod_client_ffiFFI.xcframework` and Swift wrapper are built from source (not
committed), so generate them once before the first build, and again whenever
`nod-client-core`, `nod-proto`, or `nod-client-ffi` change — the xcframework is
a prebuilt artifact, so Rust source changes don't reach the running app until
it's rebuilt:

```bash
./scripts/build-nod-client-ffi.sh
```

This needs the Rust toolchain (`rustup`); it adds the Apple targets, builds the
static libraries, and emits the xcframework + `Sources/NodClientFFI`.

Canonical local macOS app bundle build:

```bash
./scripts/build-macos-app
```

This refreshes the app you can open here:

```bash
build/DerivedData/Build/Products/Release/Nod.app
```

The script stamps the macOS bundle as `1.0 (UTC yyyymmddHHMM)` by default. To
pin a visible build number:

```bash
NOD_MAC_BUILD_NUMBER=202606101430 ./scripts/build-macos-app
```

Use this script instead of `swift build --product NodMac` when you need the
runnable `.app`. The SwiftPM command only builds `.build/.../NodMac`.

Quickest compile sanity check (no app bundle):

```bash
swift build --product NodMac
```

Compile-check iOS without signing:

```bash
xcodebuild -project Nod.xcodeproj -scheme NodIOS \
  -configuration Debug -destination 'generic/platform=iOS' \
  CODE_SIGNING_ALLOWED=NO build
```

## iOS — TestFlight

```bash
export APP_STORE_CONNECT_API_KEY_ID="..."
export APP_STORE_CONNECT_API_ISSUER_ID="..."
export APP_STORE_CONNECT_API_KEY_PATH="/path/to/AuthKey_....p8"
scripts/testflight-ios
```

Archives `NodIOS`, generates a timestamped build number, and uploads to App Store Connect. See [docs/APPLE.md](docs/APPLE.md) for options.

## macOS — signed release

`Product → Archive` in Xcode against the `NodMac` scheme, sign with your Developer ID, notarize via `notarytool`, then attach the `.dmg` (or zipped `.app`) to a GitHub Release.

## Configure a server

Open the app, tap **Add Server**, paste the server URL (e.g. `https://your-tailnet/nod`), enter a device name, and paste a user enrollment code from the server's `/admin` panel. The Devices sheet shows your registered devices and can rename or revoke them.

## Docs

- [Apple client setup](docs/APPLE.md)

## License

MIT
