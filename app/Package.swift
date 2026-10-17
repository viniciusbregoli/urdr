// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "Urdr",
    platforms: [.macOS(.v14)],
    targets: [
        .executableTarget(name: "Urdr", swiftSettings: [.swiftLanguageMode(.v5)]),
        .testTarget(name: "UrdrTests", dependencies: ["Urdr"], swiftSettings: [.swiftLanguageMode(.v5)]),
    ]
)
