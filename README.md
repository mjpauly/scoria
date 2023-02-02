# Epsilon App

## Structure

- UI, sensing modules (Swift)
    - database management, visualization, marketplace (Rust)
    - visualizations are generated in Rust and shown in a Swift WebKit WKWebView

## Toolchain Resources

- Bazel ([repo](https://github.com/bazelbuild/bazel), [docs](https://bazel.build/docs))
    - Build system used to manage the toolchain, compilation, and building at the highest level.
    - Rulesets
        - rules_xcodeproj ([repo](https://github.com/buildbuddy-io/rules_xcodeproj/),
            [docs](https://github.com/buildbuddy-io/rules_xcodeproj/tree/main/docs))
            - Bazel rules for generating the Xcode project from source files.
            - [rules_xcodeproj example](https://github.com/brentleyjones/rules_xcodeproj-demo)
        - rules_rust ([repo](https://github.com/bazelbuild/rules_rust),
            [docs](https://bazelbuild.github.io/rules_rust/))
- Rust ([docs](https://doc.rust-lang.org/book/))
    - Language that the core app functionality is written in.
    - Standard Rust features
        - LocalKey ([docs](https://doc.rust-lang.org/std/thread/struct.LocalKey.html))
            - Used to provide thread_local global variables
    - External Rust dependencies
        - sqlx ([repo](https://github.com/launchbadge/sqlx),
            [docs@0.6.2](https://docs.rs/sqlx/0.6.2/sqlx/))
            - SQLite database interface
        - tokio ([repo](https://github.com/tokio-rs/tokio),
            [docs@1.24.2](https://docs.rs/tokio/1.24.2/tokio/))
            - async runtime
        - thiserror ([repo](https://github.com/dtolnay/thiserror),
            [docs@1.0.38](https://docs.rs/thiserror/1.0.38/thiserror/))
            - custom errors with less boilerplate
        - time ([repo](https://github.com/time-rs/time),
            [docs@0.3.17](https://docs.rs/time/0.3.17/time/),
            [book](https://time-rs.github.io/book/index.html))
- Swift ([docs](https://docs.swift.org/swift-book/LanguageGuide/TheBasics.html))
    - Language that the UI and sensing modules are written in.
    - Apple Swift APIs used include SwiftUI, CoreLocation.

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
- `__test_arg=[test_fn_name]`: Run a particular test.

### Generate the Project Tree for `rust-analyzer`

Since the project isn't structured as a Cargo project, we need to generate a
`rust-project.json` for `rust-analyzer` to use. This is necessary to enable Rust
language server integrations in editors. See
[this guide](https://sharksforarms.dev/posts/neovim-rust/) for more on Neovim
language server setup in particular.

```
bazel run //src/RustLib:projtree
```

### Profiling Slow Bazel Builds, Tests, and Runs

- `--profile=profile.json`: Generate a profile of the bazel invocation. The file
is placed in the project root.
- `bazel analyze-profile profile.json`: Output a brief summary of the profile
on the command line.
- For more detail, open Google Chrome's `chrome://tracing/` page and drag the
json file onto the page to see a nice graph representation.

### Visualize Dependency Graph

Prere

```
$ bazel query 'deps(//src/App)' --output graph > graph_full.in
$ cat graph_full.in | grep -v "@" | grep -v "label" > graph_pruned.in
$ dot -Tpng < graph_pruned.in > graph_pruned.png
```

Install graphviz (includes `dot`) with `brew install graphviz`.
