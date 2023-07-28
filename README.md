# Epsilon App

## To Do

- website
    - [ ] cross compile website binary
    - [ ] try kube for scoria.app
- accessibility
    - [ ] accessible map size scaling
- debug-ability
    - [ ] action failure alterts (esp for importing)
    - [ ] ability to export stemlog for debugging
- features
    - [ ] automap to occlude unexplored areas (100m radius? diameter?)
    - [ ] zoom to current location on first install
    - [ ] quick time update buttons
- [ ] reset to "today" time range and data-centered view if user is away from
    the app for 1+ hr
- [ ] export tab on map (csv, maybe image)
- [ ] notify user when they close the app that keeping it open is required for
    data to be logged

- [ ] log more metadata
    - [ ] location config metadata (when settings were changed)
    - [ ] app usage metadata (when app launched, quit, foregrounded, backgrounded)
    - [ ] data viewership (what map data viewed and when)
    - [ ] battery charge (to correlate with location mode) [ref](https://stackoverflow.com/questions/27475506/check-battery-level-ios-swift)
- [ ] have maps\_ok respond with 204 No Content instead of 404
- [ ] custom map style that uses more black
- [ ] arcgis HD satellite maps

- later features
    - sensing: environmental noise
    - create custom markers (space / time)
         - lookup marker from public DB (apple maps?, openstreetmap?)
    - perform queries
         - visits (last time, first time, total, time spent, when visits happen)
         - traveling (different modes, time spent, num trips, when it happens, distance)
         - trends
    - improved map tile api security
        - [ ] generate keys for each new device
        - [ ] or: custom tile server

## Completed

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
- integration tests
    - backend only
        - [x] server health check works
        - [x] log_location persists data in database
        - [x] log_location sends new data to the UI
        - [x] websocket messages behave as expected
    - backend + frontend
        - [x] ui interactions produce desired effects (e.g. changing location
            parameters yields correct values from swift-facing code)
- map usability / configurability
    - [x] live map updates
    - [x] settings hidden by default, can be pulled up
    - [x] auto zoom and centering
    - [x] configurable marker color/size, map base layer
    - [x] marker colormap based on data value
    - [x] exclude data with x greater/less than x
- [x] investigate undropped websocket callbacks
- [x] secrets stored in .env
- [x] secure the UI from other apps (max 1 connection, random port, authenticate
        with number passcode)
- [x] low power modes
    - [x] reduced accuracy modes
    - [x] significant changes mode
- [x] more location diagnostics in sense tab
- [x] option to enable/disable location recording from within the app
- [x] disable server on app backgrounding, restart on foregrounding
- [x] better global UI state with yewdux
- [x] send location config when sending state from ws_session
- [x] better checkbox styling, improved location config layout, help tips
- [x] startup spash page to reduce flicker
- [x] persists certain app state values across app launches
- [x] local plotly instead of hitting cdn every app load
- [x] show data values when clicking on map
- [x] submit first app to testflight
- [x] auto location mode (switch between modes based on movement)
- [x] preserve user zoom/pan/tilt/rotate when restyling plot
- [x] refactor common code into its own module
- maplibre
    - [x] initial investigation
    - [x] update basemap
    - [x] re-center button
    - [x] colorbar
    - [x] (performance) async map updating on new data
    - [x] data point popup on click
    - [x] more clickable data points
- [x] lines between points
- [x] fully persist ui state
- [x] investigate serde default
- [x] app intoduction/tutorial on first install
- [x] create geojson in backend, serve via url
- [x] more persistently cache plotly and maplibre
- [x] decimate data if more than 50k points
- [x] ability to export/import log
- [x] 5m window for going to low accuracy in auto mode
- [x] all time button
- [x] persist time delta instead of absolute times
- [x] time of day colormapping
- [x] show colormap or not
- [x] better debug logging
- [x] Multi-page settings
- [x] ability to set preferred units
- [x] prevent horizontal app rotation
- website
    - [x] landing/marketing page
    - [x] privacy policy
    - [x] feedback/support
- [x] log: altitude, isProducedByAccessory, and isSimulatedBySoftware;
    floor, verticalAccuracy, speedAccuracy, courseAccuracy
- [x] nullable speed and course data
- [x] rust build for release (use `-c opt`)
- [x] cargo audit dependencies
- [x] was\_imported column
- [x] conditionally compile logging code
- [x] link to privacy policy and feedback form in app
- [x] prompt for desired units in introduction (and update introduction)
- [x] improved startup error handling
- [x] upgrade maptiler account
- [x] submit to app store
- [x] website terms (copyright, liability, App Store trademark notice), link to App Store
- [x] fix color slider bug by refactoring map code to have everything in the style
- [x] accessible font sizing for colorbar, attribution
- [x] custom user agent to restrict api access
    - [x] also allow only `127.0.0.1` origins

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

First, verify desired features work in dev with `bazel run //src/app/stem:dev`,
then check with the ios app using `bazel run //:iosapp`. Finally, use
`./precommit.sh` to run tests, lints, and format.

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
- cmark: produces html from markdown
    - `brew install cmark` (v0.30.0 tested)
    - Called by website

Secrets are placed in a top-level `.env` file. They are not checked into source
control; ask for them.

For compile-time checked query macros with `sqlx` we need a development database
for `sqlx` to connect to and check queries against. Run the following command:

```
bazel run //src/app/stem:db_gen
```

This is slightly suboptimal. Ideally it would integrate into the build system
automatically, getting generated anytime we compile the app. See `db_gen.rs` for
notes on this issue.

We also explicitly cache our third party javascript libraries in our source
tree. If we rely on bazel to cache it, we'll periodically need to redownload it
when unrelated build config settings change.

```
bazel run //src/app/stem/front:download_js_libs
```

### Running the App in the Simulator

```
bazel run //:iosapp
```

### Interactively Developing the UI

Run `ibazel run //src/app/stem:dev`.

Then open `localhost:8081/123` in a browser. In Firefox, the responsive web
design mode lets you change the page aspect ratio to that of a phone
(opt-cmd-M). Developer tools can be opened with opt-cmd-I.

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
- `--test_env=RUST_BACKTRACE=1`: View test backtrace when panic occurs.

```
bazel test //src/app/stem:unit_tests --test_output=all --test_arg=--nocapture --test_env=RUST_BACKTRACE=1
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

### Checking for Security Advisories

Advisories that have been checked and which have low likelihood of severe impact
are ignored. To fix a warning, temporarily add an updated version of the
dependency to the crates\_repository declaration in the WORKSPACE, then repin,
and remove it. It will stay at the new version since it's pinned.

Navigate to `src/app/stem` and run:

```
cargo audit --ignore RUSTSEC-2022-0090 --ignore RUSTSEC-2021-0065
```

Navigate to `src/server/website` and run: `cargo audit`

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

- Use `let foo = dbg!(bar)` for quicker debugging than with printing!
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

## Distribution

Note: use `-c opt` to use optimizations. [More info](https://bazel.build/docs/user-manual#compilation-mode).

1. Create the necessary certificates and provisioning profiles on
developer.apple.com.
2. Place the downloaded profile in `src/app/ios/top/`.
3. Reference the profile in the `ios_application` target in the BUILD file.
4. Build the app
    4a. Developemnt: `bazel build //:iosapp --ios_multi_cpus=arm64 -c opt`
    4b. Distribution: `bazel build //:iosapp --ios_multi_cpus=arm64 --device_debug_entitlements=false --define profile=distribution -c opt`
5. Locate the `.ipa` archive in `bazel-bin/src/app/ios/top/Epsilon.ipa`.
6. Install/upload the app
    6a. Developemnt: Go to Xcode -> devices and simulators -> [your device] ->
        `+` -> archive file
    6b. Distribution: Drag the archive into the Transporter app to upload to App
        Store Connect.

## General Troubleshooting

### Onclick Events Don't Fire on Div Padding

For some reason, on iOS onclick events don't fire when the padding of a div is
clicked, only the content. This can be fixed by changing the div into a button.
