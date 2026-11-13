// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "NotesData",
    platforms: [.iOS(.v18), .macOS(.v15)],
    products: [
        .library(name: "NotesData", targets: ["NotesData"]),
    ],
    dependencies: [
        .package(path: "../Domain"),
        .package(path: "../../../Core/Kernel"),
    ],
    targets: [
        .target(name: "NotesData", dependencies: ["NotesDomain", "Kernel"]),
    ],
    swiftLanguageModes: [.v6]
)
