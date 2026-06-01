<p align="center">
  <img src="../../assets/nod-icon.png" width="180" alt="Nod">
</p>

# nod-apple

Native SwiftUI macOS and iOS clients for [nod-server](../../server/nod-server).

One Swift package holds the shared `NodKit` library and both app shells. ~95% of the code is shared between platforms; only the `@main` entry points, entitlements, and `Info.plist` are platform-specific.

## Layout

```
Package.swift           NodKit library + SwiftPM NodMac executable (fast iteration)
Sources/NodKit/        API client, store, keychain, models, notifications, sync
Apps/
  NodMac/              macOS @main, entitlements, Info.plist, resources
  NodIOS/              iOS @main, entitlements, Info.plist, resources
  Shared/               SwiftUI views, sounds, asset catalog
Nod.xcodeproj/         Xcode targets for both apps
scripts/                Release & compliance scripts
```

## Build

Canonical local macOS app bundle build:

```bash
./scripts/build-macos-app
```

This refreshes the app you can open here:

```bash
build/DerivedData/Build/Products/Release/Nod.app
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
