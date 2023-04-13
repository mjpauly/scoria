# Epsilon App

## To Do

- [x] refactor file tree for extensibility
- [x] initial barebones yew UI
- [x] refactor to WebView (storyboard instead of swiftUI base?)
- [x] build frontend wasm with bazel
- [x] *reach feature parity with SwiftUI*
    - [x] wasm artifact building with rules_rust
    - [x] make navbar buttons bigger
    - [x] add navbar icons
    - [x] current location streaming to UI
    - [x] updates/hr streaming
    - [x] distance filter configuration
    - [x] map view with past day's data
- [x] prevent unwanted scrolling in the WKWebView
- [x] proper database migrations included in compiled source with `migrate!`
- [x] OS-assigned server port
- [_] integration tests
    - backend only
        - [x] server health check works
        - [x] log_location persists data in database
        - [x] log_location sends new data to the UI
        - [x] websocket messages behave as expected
    - backend + frontend
        - [_] ui interactions produce desired effects
    - app-level
        - [_] changing dist_filt propagates to SwiftUI
- [_] ~~CI pipeline~~
- [_] map usability / configurability
    - [x] live map updates
    - [x] settings hidden by default, can be pulled up
    - [x] auto zoom and centering
    - [x] configurable marker color/size, map base layer
    - [x] marker colormap based on data value
    - [_] exclude data with x greater/less than x
    - [_] persist selection for export + queries
- [x] investigate undropped websocket callbacks
- [x] secrets stored in .env
- [x] secure the UI from other apps (max 1 connection, random port, authenticate
        with number passcode)
- [x] low power modes
    - [x] reduced accuracy modes
    - [x] significant changes mode
- [x] more location diagnostics in sense tab
- [x] option to enable/disable location recording from within the app
- [_] ability to export log
- [_] log required: altitude, isProducedByAccessory, and isSimulatedBySoftware;
    floor, verticalAccuracy, speedAccuracy, courseAccuracy
    :: all required! don't want people to need to opt in -> reduces how much people actually collect
- [_] construct plotly plots with serde_json's Map and Value directly, to avoid
    limitations of having to define everything up front ([ref](https://stackoverflow.com/questions/59047280/how-to-build-json-arrays-or-objects-dynamically-with-serde-json))
- [_] ability to set preferred units
- [_] metadata recording of when location is enabled/disabled
- [_] handle app backgrounding/suspention
- [_] get feedback with test flight
- [_] publish to app store

- later features
    - sensing: environmental noise
    - create custom markers (space / time)
         - lookup marker from public DB (apple maps?, openstreetmap?)
    - perform queries
         - visits (last time, first time, total, time spent, when visits happen)
         - traveling (different modes, time spent, num trips, when it happens)
         - trends

## Structure

- Native Swift code interfaces with the phone's sensors and launches `stem`, the
    core app functionality.
- `stem` contains an http server which serves a web app frontend, which is shown
    in a Swift WebKit WKWebView

```
src
└── app
    ├── ios
    │   ├── sensing                     Native swift code for phone sensors
    │   ├── tests                       App integration tests
    │   ├── top                         Top-level app source
    │   └── xcodeproj                   Xcode project generator
    │       └── Epsilon.xcodeproj       The generated Xcode project
    │
    └── stem                            Rust sources for the core app functions
        ├── back                        App backend logic (database, server, ..)
        ├── dev                         Local development runner
        ├── front                       Webapp frontend for the app
        └── tests                       Stem integration tests
```

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
- Rust
    - Language that the core app functionality is written in.

- Swift
    - Language that the sensing modules are written in.

## Dev Flow

### Pre-commit checklist

Test, lint, format.

```
bazel run :dev
bazel run //:iosapp
bazel test //src/app/stem:unit_tests
bazel test //src/app/stem:int_tests --spawn_strategy=local
bazel build --aspects=@rules_rust//rust:defs.bzl%rust_clippy_aspect --output_groups=clippy_checks //...
bazel build --@rules_rust//:rustfmt.toml=//:rustfmt.toml --aspects=@rules_rust//rust:defs.bzl%rustfmt_aspect --output_groups=rustfmt_checks //...
```

### Setup

- Bazel: build system for the project
    - `brew install bazelisk`
    - Bazelisk is the launcher for Bazel. It does stuff like discover the
        desired bazel version to run from the `.bazelversion` file in the repo
        root.
- iBazel: tool for automatically rerunning a Bazel command when the sources
    change.
    - `brew install ibazel`
    - Use it just like bazel but replace `bazel` with `ibazel`.
- geckodriver: webdriver for testing the ui
    - `cargo install geckodriver`
    - Called automatically by `stem:int_tests`

Secrets are placed in a top-level `.env` file. They are not checked into source
control; ask for them.

For compile-time checked query macros with `sqlx` we need a development database
for `sqlx` to connect to and check queries against. Run the following command:

```
bazel run src/app/stem:db_gen
```

This is slightly suboptimal. Ideally it would integrate into the build system
automatically, getting generated anytime we compile the app. See `db_gen.rs` for
notes on this issue.

### Running the App in the Simulator

```
bazel run //:iosapp
```

### Interactively Developing the UI

Navigate to `src/app/stem` and run `ibazel run :dev`.

Then open `localhost:8081` in a browser. In Firefox, the responsive web design
mode lets you change the page aspect ratio to that of a phone (opt-cmd-M).

### Generate the Xcode Project

`rules_xcodeproj` is used to generate the Xcode project from Bazel BUILD files,
which serve as the declarative sources of truth. Generate the project with:

```
bazel run //:xcodeproj
```

The Xcode project itself uses Bazel for building and running the app.

### Testing the `stem` Core Library

- `bazel test //src/app/stem:unit_tests`: Run the unit tests embedded in the
    library.
- `bazel test //src/app/stem:int_tests --spawn_strategy=local`: Run the
    integration tests of the library's interface. We spawn it locally so
    geckodriver works.

Useful arguments:

- `--test_output=all`: View the output of the test produced by Cargo.
- `--test_arg=--nocapture`: Tell Cargo to show outputs from the tests.
- `--cache_test_results=no`: Rerun a test without caching.
- `--test_arg=[test_fn_name]`: Run a particular test.

```
bazel test //src/app/stem:unit_tests --test_output=all --test_arg=--nocapture --test_arg=test_get_rt
```

### Autogenerated Documentation

`bazel build //src/app/stem/front:front_doc`

### Generate the Project Tree for `rust-analyzer`

Since the project isn't structured as a Cargo project, we need to generate a
`rust-project.json` for `rust-analyzer` to use. This is necessary to enable Rust
language server integrations in editors. See
[this guide](https://sharksforarms.dev/posts/neovim-rust/) for more on Neovim
language server setup in particular.

```
bazel run //:rustanalyzer
```

### Repinning Cargo Dependencies

If the cargo dependency list in the WORKSPACE file is updated, the lockfiles
describing the exact dependency versions will need to be updated. You may see
this as an error when trying to build/run a rust rule or a rule that depends on
a rust rule. To update the lockfiles we need to explicitly "repin" the
dependencies. This is done by setting `CARGO_BAZEL_REPIN=true` for the bazel
invocation. Since we usually want to then let rust-analyzer index those
dependencies, it's convenient to combine both into one command:

```
CARGO_BAZEL_REPIN=true bazel run //:rustanalyzer
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
- Log levels from most to least important: error!, warn!, info!, debug!, trace!.

### Other tips

- Use `RUSTFLAGS=-Awarnings` to suppress warnings while working on errors.
- Never hold a synchronous lock across an `await`.
- `match` and `if let` statements will hold locks acquired in the scrutinees for
the entire arm, even if the data is cloned within the scrutinee. Get the data
in a separate variable first before putting it into the scrutinee if it's
something like an option behind the Mutex.

## Style Notes

- Check rust code formatting with: `bazel build --@rules_rust//:rustfmt.toml=//:rustfmt.toml --aspects=@rules_rust//rust:defs.bzl%rustfmt_aspect --output_groups=rustfmt_checks //...`
- Check common rust lints with: `bazel build --aspects=@rules_rust//rust:defs.bzl%rust_clippy_aspect --output_groups=clippy_checks //...`

- Code lines set to 80 characters or shorter
- Parent functions should come before the children that they call.
