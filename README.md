# Epsilon App

## Structure

- UI, sensing modules (Swift)
    - database management, visualization, marketplace (Rust)
    - visualizations are generated in Rust and shown in a Swift WebKit WKWebView

## Toolchain Resources

- Bazel ([repo](https://github.com/bazelbuild/bazel), [docs](https://bazel.build/docs))
    - Build system used to manage the toolchain, compilation, and building at the highest level.
- rules_xcodeproj ([repo](https://github.com/buildbuddy-io/rules_xcodeproj/), [docs](https://github.com/buildbuddy-io/rules_xcodeproj/tree/main/docs))
    - Bazel rules for generating the Xcode project from source files.
    - [Bazel+rules_xcodeproj example](https://github.com/brentleyjones/rules_xcodeproj-demo)
- Swift ([docs](https://docs.swift.org/swift-book/LanguageGuide/TheBasics.html))
    - Language that the UI and sensing modules are written in.
    - Apple Swift APIs used include SwiftUI, CoreLocation.
- Rust ([docs](https://doc.rust-lang.org/book/))
    - Language that the core app functionality is written in.

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

### Testing RustLib

```
bazel test //src/RustLib:tests
```

To view the output of the test produced by Cargo add the argument
`--test_output=all`. To then also tell Cargo to show outputs from the tests,
pass in `--test_arg=--nocapture`. To rerun a test without caching use
`--cache_test_results=no`.

