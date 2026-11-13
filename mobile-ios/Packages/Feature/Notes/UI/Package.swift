// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "NotesUI",
    platforms: [.iOS(.v18), .macOS(.v15)],
    products: [
        .library(name: "NotesUI", targets: ["NotesUI"]),
    ],
    dependencies: [
        .package(path: "../Contracts"),
        .package(path: "../Domain"),
        .package(path: "../../../Core/Kernel"),
        .package(path: "../../../Core/UICore"),
    ],
    targets: [
        .target(
            name: "NotesUI",
            dependencies: ["NotesContracts", "NotesDomain", "Kernel", "UICore"]
        ),
        .testTarget(
            name: "NotesUITests",
            dependencies: ["NotesUI", "NotesDomain", "Kernel", "UICore"]
        ),
    ],
    swiftLanguageModes: [.v6]
)
