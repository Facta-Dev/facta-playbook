// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "UICore",
    platforms: [.iOS(.v18), .macOS(.v15)],
    products: [
        .library(name: "UICore", targets: ["UICore"]),
    ],
    dependencies: [
        .package(path: "../Kernel"),
    ],
    targets: [
        .target(name: "UICore", dependencies: ["Kernel"]),
    ],
    swiftLanguageModes: [.v6]
)
