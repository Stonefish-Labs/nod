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
            path: "Sources/NodKit",
            linkerSettings: [
                .linkedFramework("DeviceCheck"),
                .linkedFramework("Security"),
                .linkedFramework("UserNotifications")
            ]
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
