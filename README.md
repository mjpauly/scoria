# Epsilon App

## Structure

- UI, sensing modules (Swift)
    - database management, visualization, marketplace (Rust)
    - visualizations are generated in Rust and shown in a Swift WebKit WKWebView

## Toolchain Resources

- Bazel ([repo](https://github.com/bazelbuild/bazel), [docs](https://bazel.build/docs))
    - Build system used to manage the toolchain, compilation, and building at the highest level.
- Swift ([docs](https://docs.swift.org/swift-book/LanguageGuide/TheBasics.html))
    - Language that the UI and sensing modules are written in.
    - Apple Swift APIs used include SwiftUI, CoreLocation.
- Rust ([docs](https://doc.rust-lang.org/book/))
    - Language that the core app functionality is written in.
- Bazel rulesets
    - rules_xcodeproj ([repo](https://github.com/buildbuddy-io/rules_xcodeproj/), [docs](https://github.com/buildbuddy-io/rules_xcodeproj/tree/main/docs))
        - Bazel rules for generating the Xcode project from source files.
        - [Bazel+rules_xcodeproj example](https://github.com/brentleyjones/rules_xcodeproj-demo)
    - rules_rust ([repo](https://github.com/bazelbuild/rules_rust), [docs](https://bazelbuild.github.io/rules_rust/))
- External Rust dependencies
    - sqlx ([repo](https://github.com/launchbadge/sqlx))
        - SQLite database interface
    - tokio ([repo](https://github.com/tokio-rs/tokio))
- Rust features
    - LocalKey ([docs](https://doc.rust-lang.org/std/thread/struct.LocalKey.html))
        - Used to provide thread_local global variables

## Dev Flow

### Running the App in the Simulator

```
bazel run //src/App
```

### Generate the Xcode Project

`rules_xcodeproj` is used to generate the Xcode project from Bazel BUILD files,
which serve as the declarative sources of truth. Generate the project with:

```
bazel run //:xcodeproj
```

The Xcode project itself uses Bazel for building and running the app.

### Testing the RustLib Core Library

- `bazel test //src/RustLib:unit_tests`: Run the unit tests embedded in the library.
- `bazel test //src/RustLib:int_tests`: Run the integration tests of the library's interface.
- `bazel test //src/RustLib:all`: Run both the unit tests and the integration tests.

Useful arguments:

- `--test_output=all`: View the output of the test produced by Cargo.
- `--test_arg=--nocapture`: Tell Cargo to show outputs from the tests.
- `--cache_test_results=no`: Rerun a test without caching.
