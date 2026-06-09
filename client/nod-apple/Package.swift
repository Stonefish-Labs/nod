// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "NodApple",
    platforms: [
        .iOS(.v17),
        .macOS(.v14),
        .watchOS(.v10)
    ],
    products: [
        .library(name: "NodKit", targets: ["NodKit"]),
        .executable(name: "NodMac", targets: ["NodMac"])
    ],
    targets: [
        .target(
            name: "NodKit",
            dependencies: ["NodProtoFFI"],
            path: "Sources/NodKit",
            linkerSettings: [
                .linkedFramework("DeviceCheck"),
                .linkedFramework("Security"),
                .linkedFramework("UserNotifications")
            ]
        ),
        // Generated UniFFI Swift wrapper over the Rust signing crypto. Built by
        // scripts/build-nod-proto-ffi.sh.
        .target(
            name: "NodProtoFFI",
            dependencies: ["nod_proto_ffiFFI"],
            path: "Sources/NodProtoFFI",
            // UniFFI 0.28's generated wrapper predates Swift 6 strict concurrency
            // (a global `var initializationResult`). Build this generated shim in
            // Swift 5 language mode; NodKit and the apps stay on Swift 6.
            swiftSettings: [.swiftLanguageMode(.v5)]
        ),
        .binaryTarget(
            name: "nod_proto_ffiFFI",
            path: "Frameworks/nod_proto_ffiFFI.xcframework"
        ),
        .executableTarget(
            name: "NodMac",
            dependencies: ["NodKit"],
            path: "Apps",
            exclude: [
                "NodIOS",
                "Shared/Assets.xcassets",
                "Shared/AppIcon60x60@3x.png",
                "NodMac/Resources",
                "NodMac/NodMac.entitlements"
            ],
            resources: [
                .copy("Shared/Sounds")
            ],
            linkerSettings: [
                .linkedFramework("AppKit"),
                .linkedFramework("SwiftUI")
            ]
        ),
        .testTarget(
            name: "NodKitTests",
            dependencies: ["NodKit"],
            path: "Tests/NodKitTests"
        )
    ]
)
