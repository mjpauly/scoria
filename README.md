# Epsilon App

## To Do

- [x] refactor file tree for extensibility
- [x] initial barebones yew UI
- [x] refactor to WebView (storyboard instead of swiftUI base?)
- [_] *reach feature parity with SwiftUI*
- [_] CI pipeline
- [_] integration tests
- [_] secure the UI from other apps (max 1 connection, random port, authenticate
        with number passcode, shut down when not in use)

- proper database migrations included in compiled source with `migrate!` macro
- new features
    - scatter plot:
         - colorscale based on data value
         - marker size
         - nice way to close the WebView
    - sensing: environmental noise
    - create custom markers (space / time)
         - lookup marker from public DB (apple maps?, openstreetmap?)
    - perform queries
         - visits (last time, first time, total, time spent, when visits happen)
         - traveling (different modes, time spent, num trips, when it happens)
         - trends
- `stem` integration tests
- App integration tests
- more battery-efficient data collection

- [_] wasm artifact building with rules_rust

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
        - time ([repo](https://github.com/time-rs/time),
            [docs@0.3.17](https://docs.rs/time/0.3.17/time/),
            [book](https://time-rs.github.io/book/index.html))
            - used for timestamping measurements in the database
            - not imported on its own, but with `sqlx::types::time`
            - exact version used with `sqlx` might be different than the
                versions linked
        - plotly ([repo](https://github.com/igiagkiozis/plotly),
            [docs@0.8.3](https://docs.rs/plotly/0.8.3/plotly/))
            - used to generate visualizations
            - doesn't currently support compiling for iOS, so it is vendored in
                at this repo: https://github.com/mjpauly/plotly/

- Swift ([docs](https://docs.swift.org/swift-book/LanguageGuide/TheBasics.html))
    - Language that the UI and sensing modules are written in.
    - Apple Swift APIs used include SwiftUI, CoreLocation.

## Dev Flow

### Setup

Some tools used come from the system rather than a Bazel toolchain. This is a
temporary workaround where the Bazel toolchain was challenging to implement.
These need to be installed separately.

- Trunk: used to build the frontend webassembly application
    - First install Rust onto your system, then do
        `cargo install --locked --version 0.16.0 trunk`
- tailwindcss: used for CSS utility classes in the UI
    - install npm/node with `brew install npm node`
    - install tailwind cli with `npm install -g tailwindcss`
    - instructions [here](https://tailwindcss.com/docs/installation)

### Running the App in the Simulator

```
bazel run //:iosapp
```

### Interactively Developing the UI

Navigate to `src/app/stem/front` and run these commands in separate terminals:

```
npx tailwindcss -i ./styles/input.css -o ./styles/output.css --watch
trunk serve --open
```

Then open `localhost:8080` in a browser. In Firefox, the responsive web design
mode lets you change the page aspect ratio to that of a phone.

### Generate the Xcode Project

`rules_xcodeproj` is used to generate the Xcode project from Bazel BUILD files,
which serve as the declarative sources of truth. Generate the project with:

```
bazel run //:xcodeproj
```

The Xcode project itself uses Bazel for building and running the app.

### Testing the `stem` Core Library

- `bazel test //src/app/stem:unit_tests`: Run the unit tests embedded in the library.
- `bazel test //src/app/stem:int_tests`: Run the integration tests of the library's interface.
- `bazel test //src/app/stem:all`: Run both the unit tests and the integration tests.

Useful arguments:

- `--test_output=all`: View the output of the test produced by Cargo.
- `--test_arg=--nocapture`: Tell Cargo to show outputs from the tests.
- `--cache_test_results=no`: Rerun a test without caching.
- `--test_arg=[test_fn_name]`: Run a particular test.

```
bazel test //src/app/stem:unit_tests --test_output=all --test_arg=--nocapture --test_arg=test_get_rt
```

### Generate the Project Tree for `rust-analyzer`

Since the project isn't structured as a Cargo project, we need to generate a
`rust-project.json` for `rust-analyzer` to use. This is necessary to enable Rust
language server integrations in editors. See
[this guide](https://sharksforarms.dev/posts/neovim-rust/) for more on Neovim
language server setup in particular.

```
bazel run //:rustanalyzer
```

### Profiling Slow Bazel Builds, Tests, and Runs

- `time bazel build ...`: View how long you're actually waiting.
- `--profile=profile.json`: Generate a profile of the bazel invocation. The file
is placed in the project root.
- `bazel analyze-profile profile.json`: Output a brief summary of the profile
on the command line.
- For more detail, open Google Chrome's `chrome://tracing/` page and drag the
json file onto the page to see a nice graph representation.

### Visualize Dependency Graph

```
$ bazel query 'deps(//:iosapp)' --output graph > graph_full.in
$ cat graph_full.in | grep -v "@" | grep -v "label" > graph_pruned.in
$ dot -Tpng < graph_pruned.in > graph_pruned.png
```

Prereq: install graphviz (includes `dot`) with `brew install graphviz`.

### Rust Debug Output

- Use the `dbg!(var_name)` macro for quicker debugging than with printing!
- `.unwrap_or_else(|err| { println("got error {}", err); return; })`
    - if return type is `()` on success: `if let Err(e) = run(config) {`
- `eprintln` for stderr
