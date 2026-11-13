// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "Kernel",
    platforms: [.iOS(.v18), .macOS(.v15)],
    products: [
        .library(name: "Kernel", targets: ["Kernel"]),
    ],
    targets: [
        .target(name: "Kernel"),
        .testTarget(name: "KernelTests", dependencies: ["Kernel"]),
    ],
    swiftLanguageModes: [.v6]
)
