// swift-tools-version: 5.6
// The swift-tools-version declares the minimum version of Swift required to build this package.

import PackageDescription

let package = Package(
    name: "WysiwygComposer",
    platforms: [
        .iOS(.v14)
    ],
    products: [
        .library(
            name: "WysiwygComposer",
            targets: ["WysiwygComposer"]
        ),
    ],
    targets: [
        .binaryTarget(
            name: "WysiwygComposerFFI",
            path: "WysiwygComposerFFI.xcframework"
        ),
        .target(
            name: "WysiwygComposer",
            dependencies: [
                .target(name: "WysiwygComposerFFI")
            ]
        ),
        .testTarget(
            name: "WysiwygComposerTests",
            dependencies: ["WysiwygComposer"]),
    ]
)
