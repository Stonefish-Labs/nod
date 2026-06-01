# Agent Build Instructions

- To rebuild the runnable macOS app bundle, run `./scripts/build-macos-app` from this directory.
- The expected local app output is `build/DerivedData/Build/Products/Release/Nod.app`.
- Do not use `swift build --product NodMac` when the user asks to rebuild or run the macOS app. That command only builds the SwiftPM executable at `.build/.../NodMac`, not `Nod.app`.
- Do not use `xcodebuild -scheme NodMac` for local app-bundle refreshes. The scheme name collides with the Swift package product and can build the wrong artifact.
- `swift build --product NodMac` is only a quick compile check.
