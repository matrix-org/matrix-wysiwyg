# `wysiwyg-ffi`

Rust code that can be used to generate Kotlin and Swift bindings for
wysiwyg-rust.

## Building

### Android

* [Install Rust](https://www.rust-lang.org/tools/install)
* [Install Android NDK](https://developer.android.com/ndk/downloads) - download
  android-ndk-r22b-linux-x86_64.zip and unzip it e.g.:

```bash
unzip android-ndk-r22b-linux-x86_64.zip
```

NOTE: at time of writing (2022-06-28) you needed to use android-ndk-r22b or
earlier, because later versions fail with an error like
`ld: error: unable to find library -lgcc`.  See
[rust-lang #85806](https://github.com/rust-lang/rust/pull/85806) for more.

* Configure Rust for cross compilation:

```bash
rustup target add aarch64-linux-android
rustup target add x86_64-linux-android
rustup target add i686-linux-android
rustup target add armv7-linux-androideabi
```

(Note: `aarch64` is for most physical Android devices, but `x86_64` is useful
for running an Android emulator on a PC. You can also add `i686` if you use
32-bit emulators.  `armv7` and `i686` are for older devices, but they can be
used by default in the Android Studio build, so it's useful to have it
available.)

* Edit Cargo config `bindings/wysiwyg-ffi/.cargo/config.toml` to contain
  something like:

```toml
[target.aarch64-linux-android]
ar = "NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin/ar"
linker = "NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android30-clang"

[target.x86_64-linux-android]
ar = "NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin/ar"
linker = "NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin/x86_64-linux-android30-clang"
```

Replacing NDK_HOME with something like `/home/andy/android-ndk-r22b/`.

(Again, add the equivalent for `i686` if you use a 32-bit emulator.)

(Note: this file is ignored in the top-level `.gitignore` file, so it won't be
checked in to Git.)

(More details in the
[Cargo reference](https://doc.rust-lang.org/cargo/reference/config.html)).

* Install uniffi-bindgen:

```bash
cargo install uniffi_bindgen
```

Make sure you install same version of uniffi_bindgen that is mentioned in
`bindings/wysiwyg-ffi/Cargo.toml`, otherwise you will get weird errors.

* Build shared object:

```bash
cd bindings/wysiwyg-ffi
cargo build --target x86_64-linux-android --release
```

This will create:

```
../../target/x86_64-linux-android/release/libuniffi_wysiwyg_composer.so
```

* Strip the libraries to make them smaller:

```bash
NDK_HOME/toolchains/x86_64-4.9/prebuilt/linux-x86_64/bin/x86_64-linux-android-strip \
    ../../target/x86_64-linux-android/debug/libuniffi_wysiwyg_composer.so
```

Replacing NDK_HOME with something like `/home/andy/android-ndk-r22b/`.

See ../../examples/example-android for a Gradle project that runs the above
and includes the built library in a real Android app.

### iOS

* [Install Rust](https://www.rust-lang.org/tools/install)

* Configure Rust for cross compilation:

```bash
rustup target add aarch64-apple-ios
rustup target add aarch64-apple-ios-sim
rustup target add x86_64-apple-ios
```

* Install uniffi-bindgen

⚠️  Check bindings/wysiwyg-ffi/Cargo.toml for the version of uniffi. You MUST
use the same version here!

```bash
cargo install uniffi_bindgen --version 0.19.2
```

* Build shared object:

```bash
cd bindings/ffi
cargo build --release --target aarch64-apple-ios
cargo build --release --target aarch64-apple-ios-sim
cargo build --release --target x86_64-apple-ios

mkdir -p ../../target/ios-simulator
lipo -create \
  ../../target/x86_64-apple-ios/release/libwysiwyg_ffi.a \
  ../../target/aarch64-apple-ios-sim/release/libwysiwyg_ffi.a \
  -output ../../target/ios-simulator/libwysiwyg_ffi.a
```

* This will create static libraries for both iOS devices and simulators:

```
../../target/x86_64-apple-ios/debug/libwysiwyg_ffi.a
../../target/ios-simulator/libwysiwyg_ffi.a
```

* Generate the bindings inside given output dir:

⚠️ The installed version should always match the version used by Cargo, see
`Cargo.toml` content inside this directory to retrieve it and use
`cargo install uniffi_bindgen --version <VERSION_NUMBER>` if needed.

```bash
uniffi-bindgen \
    generate src/wysiwyg_composer.udl \
    --language swift \
    --config uniffi.toml \
    --out-dir MY_OUTPUT_DIR

cd MY_OUTPUT_DIR
mv *.h         headers/
mv *.modulemap headers/module.modulemap
mv *.swift     Sources/
```

Note: The project should always have a single Swift modulemap file, and it
should be named `module.modulemap` otherwise the generated framework will not
expose symbols properly to Swift.

* Generate the .xcframework file:

```bash
xcodebuild -create-xcframework \
  -library ../../target/aarch64-apple-ios/debug/libwysiwyg_composer.a" \
  -headers MY_OUTPUT_DIR/headers \
  -library MY_OUTPUT_DIR/libwysiwyg_composer.a" \
  -headers MY_OUTPUT_DIR/headers \
  -output MY_OUTPUT_DIR/ExampleRustBindings.xcframework"
```

Now you can use the framework wherever you see fit, just add the framework (as
well as the generated Swift code from `MY_OUTPUT_DIR/Sources/`) as a dependency
inside an existing XCode project, or package them with your preferred dependency
manager. See
[Example Rust Bindings](https://gitlab.com/andybalaam/example-rust-bindings/)
for a full example of how it all fits together.
