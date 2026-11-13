// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "NotesDomain",
    platforms: [.iOS(.v18), .macOS(.v15)],
    products: [
        .library(name: "NotesDomain", targets: ["NotesDomain"]),
    ],
    dependencies: [
        .package(path: "../../../Core/Kernel"),
    ],
    targets: [
        .target(name: "NotesDomain", dependencies: ["Kernel"]),
        .testTarget(name: "NotesDomainTests", dependencies: ["NotesDomain"]),
    ],
    swiftLanguageModes: [.v6]
)
