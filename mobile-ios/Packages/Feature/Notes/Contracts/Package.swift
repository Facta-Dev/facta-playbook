// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "NotesContracts",
    platforms: [.iOS(.v18), .macOS(.v15)],
    products: [
        .library(name: "NotesContracts", targets: ["NotesContracts"]),
    ],
    targets: [
        .target(name: "NotesContracts"),
    ],
    swiftLanguageModes: [.v6]
)
