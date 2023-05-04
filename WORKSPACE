load("@bazel_tools//tools/build_defs/repo:http.bzl", "http_archive")


# rules_xcodeproj

http_archive(
    name = "rules_xcodeproj",
    sha256 = "54cee524abd72db950482ded168dd44397369077b734d4cf4b06f734e10a3e80",
    url = "https://github.com/MobileNativeFoundation/rules_xcodeproj/releases/download/1.5.1/release.tar.gz",
)

load(
    "@rules_xcodeproj//xcodeproj:repositories.bzl",
    "xcodeproj_rules_dependencies",
)

xcodeproj_rules_dependencies()


# apple / swift support

http_archive(
    name = "build_bazel_rules_apple",
    sha256 = "9e26307516c4d5f2ad4aee90ac01eb8cd31f9b8d6ea93619fc64b3cbc81b0944",
    url = "https://github.com/bazelbuild/rules_apple/releases/download/2.2.0/rules_apple.2.2.0.tar.gz",
)

load(
    "@build_bazel_rules_apple//apple:repositories.bzl",
    "apple_rules_dependencies",
)

apple_rules_dependencies()

load(
    "@build_bazel_rules_swift//swift:repositories.bzl",
    "swift_rules_dependencies",
)

swift_rules_dependencies()

load(
    "@build_bazel_rules_swift//swift:extras.bzl",
    "swift_rules_extra_dependencies",
)

swift_rules_extra_dependencies()

load(
    "@build_bazel_apple_support//lib:repositories.bzl",
    "apple_support_dependencies",
)

apple_support_dependencies()

# provisioning profile

load(
    "@build_bazel_rules_apple//apple:apple.bzl",
    "provisioning_profile_repository",
)

provisioning_profile_repository(
    name = "local_provisioning_profiles",
)


# rules_rust

http_archive(
    name = "rules_rust",
    sha256 = "950a3ad4166ae60c8ccd628d1a8e64396106e7f98361ebe91b0bcfe60d8e4b60",
    urls = ["https://github.com/bazelbuild/rules_rust/releases/download/0.20.0/rules_rust-v0.20.0.tar.gz"],
)

load(
    "@rules_rust//rust:repositories.bzl",
    "rules_rust_dependencies",
    "rust_register_toolchains",
)

rules_rust_dependencies()

rust_register_toolchains(
    edition = "2021",
    versions = ["1.66.1"],
    extra_target_triples = [
        "aarch64-apple-ios-sim",
        "aarch64-apple-ios",
        "x86_64-apple-ios",
        "wasm32-unknown-unknown",
    ],
)

# WASM for the webapp-based mobile UI

load("@rules_rust//wasm_bindgen:repositories.bzl", "rust_wasm_bindgen_repositories")

rust_wasm_bindgen_repositories()


# crate universe
# manage external Rust dependencies using the Crate Universe rules
# docs: https://bazelbuild.github.io/rules_rust/crate_universe.html
# example: https://github.com/bazelbuild/rules_rust/tree/main/examples/crate_universe/no_cargo_manifests

load("@rules_rust//crate_universe:repositories.bzl", "crate_universe_dependencies")

crate_universe_dependencies()

load("@rules_rust//crate_universe:defs.bzl", "crate", "crates_repository", "render_config", "splicing_config")

# crate repository for the `stem` core of the app
# NOTE: don't forget to add the dependency to the rust_library target!
# e.g. deps = [ ... , "@stem_crate_index//:tokio", ]
crates_repository(
    name = "stem_crate_index",
    cargo_lockfile = "//src/app/stem:Cargo.lock",
    lockfile = "//src/app/stem:Cargo.Bazel.lock",
    isolated = False,  # cache results of the previous invocation to
                       # ${HOME}/.cargo so using it is fast
    packages = {
        "rand": crate.spec(version = "0.8.5"),
        "sqlx": crate.spec(
            version = "0.6.2",
            features = ["runtime-tokio-native-tls", "sqlite", "time", "macros"],
        ),
        "tokio": crate.spec(
            version = "1.24.2",
            features = ["full"],
        ),
        "anyhow": crate.spec(
            version = "1.0.68",
        ),
        "actix-web": crate.spec(
            version = "4.3.0",
        ),
        "actix": crate.spec(
            version = "0.13.0",
        ),
        "actix-web-actors": crate.spec(
            version = "4.2.0",
        ),
        "actix-files": crate.spec(
            version = "0.6.2",
        ),
        "serde": crate.spec(
            version = "1.0.152",
        ),
        "bincode": crate.spec(version = "1.3.3"),
        "once_cell": crate.spec(version = "1.17.1"),
        "zip": crate.spec(
            version = "0.6.4",
        ),

        # frontend
        "dotenvy_macro": crate.spec(version = "0.15.7",),
        "humantime": crate.spec(version = "2.1.0",),
        "js-sys": crate.spec(version = "0.3.60",), # tied to wasm-bindgen 0.2.83
        "obfstr": crate.spec(version = "0.4.3",),
        # "plotly": crate.spec(version = "0.8.3", features = ["wasm"],),
        "plotly": crate.spec(
            git = "https://github.com/mjpauly/plotly",
            branch = "all_new_features",
            features = ["wasm"],
        ),
        "serde_json": crate.spec(version = "1.0.94",),
        "serde-wasm-bindgen": crate.spec(version = "0.5.0",),
        "uom": crate.spec(version = "0.34.0"),
        "yew": crate.spec(
            version = "0.20.0",
            features = ["csr"],
        ),
        "yew-hooks": crate.spec( version = "0.2.0",),
        "yew-router": crate.spec(
            version = "0.17.0",
        ),
        "yewdux": crate.spec( version = "0.9.2",),
        "time": crate.spec(
            version = "0.3.20",
            features = ["local-offset", "wasm-bindgen", "formatting", "parsing",
                        "serde",
            ],
        ),
        "futures": crate.spec(version = "0.3.26"),
        "gloo-net": crate.spec(version = "0.2.6"),
        "wasm-bindgen-futures": crate.spec(version = "0.4.33"),
        "getrandom": crate.spec( # dependency of uuid
            version = "0.2.8",
            features = ["js"]
        ),
        "uuid": crate.spec(
            version = "1.3.0",
            features = ["v4", "fast-rng", "macro-diagnostics",]
        ),
        "web-sys": crate.spec(  # last web-sys ok with wasm-bindgen 0.2.83
            version = "0.3.60",
            features = [
                "Performance", "Window", "HtmlInputElement", "Location",
                "HtmlSelectElement", "Document", "Element",
            ],
        ),
        "log": crate.spec(
            version = "0.4.17",
        ),
        "wasm-logger": crate.spec(
            version = "0.2.0",
        ),
        "yew_icons": crate.spec(
            version = "0.7.2",
            # git = "https://github.com/mjpauly/yew_icons",
            # branch = "main",
            features = [
                "BootstrapBrush",
                "BootstrapCalendarRange",
                "BootstrapExclamationTriangle",
                "BootstrapFilter",
                "BootstrapFunnel",
                "BootstrapGlobeAmericas",
                "BootstrapJournal",
                "BootstrapJournalText",
                "BootstrapMap",
                "BootstrapPlusLg",
                "BootstrapQuestionCircle",
                "BootstrapSave",
                "BootstrapSoundwave",
                "BootstrapTools",
                "BootstrapXCircle",
            ],
        ),
        # Unresolved bug if we upgrade to wasm-bindgen 0.2.84, probably because
        # the wasm_bindgen rules in rules_rust are not updated yet, and the
        # versions used in the CLI and in the code need to be synced. So we pin
        # it at 0.2.83.
        # Info: https://github.com/rustwasm/wasm-bindgen/issues/2776
        "wasm-bindgen": crate.spec(
            version = "=0.2.83",
        ),

        # dev + testing
        "env_logger": crate.spec(version = "0.10.0",),
        "reqwest": crate.spec(version = "0.11.15",),
        "tokio-tungstenite": crate.spec(version = "0.18.0",),
        "futures-util": crate.spec(version = "0.3.27",),
        "rusty-fork": crate.spec(version = "0.3.0",),
        "fantoccini": crate.spec(version = "0.19.3",),
    },
    splicing_config = splicing_config(resolver_version = "2"),

    # Setting the default package name to `""` forces the use of the macros defined in this repository
    # to always use the root package when looking for dependencies or aliases. This should be considered
    # optional as the repository also exposes alises for easy access to all dependencies.
    render_config = render_config(
        default_package_name = ""
    ),
)

load("@stem_crate_index//:defs.bzl", "crate_repositories")

crate_repositories()


# rust analyzer
# Our project isn't structured as a Cargo project, so we need to generate the
# rust-project.json for rust-analyzer to use.
# The rust-analyzer binary must be in your path.

load("@rules_rust//tools/rust_analyzer:deps.bzl", "rust_analyzer_dependencies")

rust_analyzer_dependencies()


# ==============================================================================

# packaging rules for bundling the UI
http_archive(
    name = "rules_pkg",
    urls = [
        "https://mirror.bazel.build/github.com/bazelbuild/rules_pkg/releases/download/0.8.1/rules_pkg-0.8.1.tar.gz",
        "https://github.com/bazelbuild/rules_pkg/releases/download/0.8.1/rules_pkg-0.8.1.tar.gz",
    ],
    sha256 = "8c20f74bca25d2d442b327ae26768c02cf3c99e93fad0381f32be9aab1967675",
)
load("@rules_pkg//:deps.bzl", "rules_pkg_dependencies")
rules_pkg_dependencies()

# ==============================================================================

# nodejs rules for tailwindcss

http_archive(
    name = "build_bazel_rules_nodejs",
    sha256 = "f10a3a12894fc3c9bf578ee5a5691769f6805c4be84359681a785a0c12e8d2b6",
    urls = ["https://github.com/bazelbuild/rules_nodejs/releases/download/5.5.3/rules_nodejs-5.5.3.tar.gz"],
)

load("@build_bazel_rules_nodejs//:repositories.bzl", "build_bazel_rules_nodejs_dependencies")

build_bazel_rules_nodejs_dependencies()

load("@build_bazel_rules_nodejs//:index.bzl", "node_repositories", "yarn_install")

node_repositories()

yarn_install(
    name = "front_npm",
    package_json = "//src/app/stem/front:package.json",
    yarn_lock = "//src/app/stem/front:yarn.lock",
    frozen_lockfile = False,
)
