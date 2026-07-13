#!/bin/bash

# Build the dylib
cargo build

# Generate bindings
cargo run --bin uniffi-bindgen generate --library ./target/debug/libengine.dylib --language swift --out-dir ./bindings

# Add all targets and build
for TARGET in \
    aarch64-apple-darwin \
    aarch64-apple-ios \
    aarch64-apple-ios-sim \
    x86_64-apple-ios
do
    rustup target add $TARGET
    cargo build --release --target=$TARGET
done

# Rename *.modulemap to module.modulemap
mv ./bindings/engineFFI.modulemap ./bindings/module.modulemap

# ── iOS ────────────────────────────────────────────────────────────────────────

# Move the Swift file to the iOS project
rm -f ./ios/airbridge/airbridge/engine.swift
cp ./bindings/engine.swift ./ios/airbridge/airbridge/engine.swift

# Recreate iOS XCFramework
rm -rf "ios/airbridge/Engine.xcframework"
xcodebuild -create-xcframework \
    -library ./target/aarch64-apple-ios-sim/release/libengine.a -headers ./bindings \
    -library ./target/aarch64-apple-ios/release/libengine.a -headers ./bindings \
    -output "ios/airbridge/Engine.xcframework"

# ── macOS ──────────────────────────────────────────────────────────────────────

# Move the Swift file to the macOS project
rm -f ./macos/airbridge/airbridge/engine.swift
mv ./bindings/engine.swift ./macos/airbridge/airbridge/engine.swift

# Recreate macOS XCFramework
rm -rf "macos/airbridge/Engine.xcframework"
xcodebuild -create-xcframework \
    -library ./target/aarch64-apple-darwin/release/libengine.a -headers ./bindings \
    -output "macos/airbridge/Engine.xcframework"

# ── Cleanup ────────────────────────────────────────────────────────────────────
rm -rf bindings