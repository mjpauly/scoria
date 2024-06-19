load("@bazel_tools//tools/build_defs/repo:http.bzl", "http_archive")


# rules_xcodeproj

http_archive(
    name = "rules_xcodeproj",
    sha256 = "f5c1f4bea9f00732ef9d54d333d9819d574de7020dbd9d081074232b93c10b2c",
    url = "https://github.com/MobileNativeFoundation/rules_xcodeproj/releases/download/1.13.0/release.tar.gz",
)

load(
    "@rules_xcodeproj//xcodeproj:repositories.bzl",
    "xcodeproj_rules_dependencies",
)

xcodeproj_rules_dependencies()

load("@bazel_features//:deps.bzl", "bazel_features_deps")

bazel_features_deps()


# apple / swift support

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
    integrity = "sha256-GuRaQT0LlDOYcyDfKtQQ22oV+vtsiM8P0b87qsvoJts=",
    urls = ["https://github.com/bazelbuild/rules_rust/releases/download/0.39.0/rules_rust-v0.39.0.tar.gz"],
)

load(
    "@rules_rust//rust:repositories.bzl",
    "rules_rust_dependencies",
    "rust_register_toolchains",
)

rules_rust_dependencies()

rust_register_toolchains(
    edition = "2021",
    versions = ["1.74.0"],
    extra_target_triples = [
        "aarch64-apple-ios-sim",
        "aarch64-apple-ios",
        "x86_64-apple-ios",
        "wasm32-unknown-unknown",
        "x86_64-unknown-linux-gnu",
        "aarch64-linux-android",
        "armv7-linux-androideabi",
        "i686-linux-android",
        "x86_64-linux-android",
    ],
)

# WASM for the webapp-based mobile UI

load("@rules_rust//wasm_bindgen:repositories.bzl", "rust_wasm_bindgen_dependencies", "rust_wasm_bindgen_register_toolchains")

rust_wasm_bindgen_dependencies()

rust_wasm_bindgen_register_toolchains()


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
    cargo_lockfile = "//src/app/stem/crates:Cargo.lock",
    lockfile = "//src/app/stem/crates:Cargo.Bazel.lock",
    isolated = False,  # cache results of the previous invocation to
                       # ${HOME}/.cargo so using it is fast
    # Patches:
    # If changes in fork have been committed, generate the patch with
    # $ git format-patch --keep-subject --no-stat --zero-commit origin/main
    # more info: https://brentley.dev/patching-bazel-external-dependencies/
    annotations = {
        # Patch-in iOS support in the clipper-sys build script (dep of
        # geo-clipper)
        "clipper-sys": [crate.annotation(
            patches = ["@//src/app/stem/crates:clipper_sys_ios.patch"],
            patch_args = ["-p1"],
        )],
    },
    packages = {
        "approx": crate.spec(version = "0.5.1"),
        "base64": crate.spec(version = "0.22.0"),
        "coord_transforms": crate.spec(version = "1.4.0"),
        "csv": crate.spec(version = "1.3.0"),
        "futures-core": crate.spec(version = "0.3.28"),
        "geo": crate.spec(version = "0.26.0"),
        "geo-clipper": crate.spec(version = "0.7.3"),
        "geojson": crate.spec(version = "0.24.1", features = ["geo-types"]),
        "gpx": crate.spec(version = "0.9.1"),
        "itertools": crate.spec(version = "0.13.0"),
        "log-panics": crate.spec(version = "2.1.0"),
        "mvt": crate.spec(version = "0.8.1"),
        "nalgebra": crate.spec(version = "0.31.4"), # for coord_transforms
        "nav-types": crate.spec(version = "0.5.2"),
        "pointy": crate.spec(version = "0.4.0"),
        "rand": crate.spec(version = "0.8.5"),
        "reqwest": crate.spec(
            version = "0.11.15",
            # features = ["gzip", "deflate", "brotli"]
            # toggle the prev line and the next 2 lines to use system-provided
            # TLS for iOS (needed to distribute in France until approved)
            # No OpenSSL install or CA certs are easily found on Andriod, so
            # rustls with webpki roots is used.
            features = ["gzip", "deflate", "brotli", "rustls-tls-webpki-roots"],
            default_features = False, # disable OpenSSL
        ),
        "sqlx": crate.spec(
            version = "0.6.2",
            # features = ["runtime-tokio-native-tls", "sqlite", "time", "macros"],
            # toggle the next/prev lines to use system-provided TLS for iOS
            features = ["runtime-tokio-rustls", "sqlite", "time", "macros"],
        ),
        "strum": crate.spec(
            version = "0.25.0",
            features = ["derive"],
        ),
        "tokio": crate.spec(
            version = "1.24.2",
            features = ["full"],
        ),
        "tracing": crate.spec(
            version = "0.1.37",
            # statically remove tracing instrumentation at levels above error
            # for release builds
            features = ["release_max_level_error"],
        ),
        "tracing-appender": crate.spec(version = "0.2.3"),
        "tracing-subscriber": crate.spec(
            version = "0.3.17",
            features = ["env-filter", "tracing-log"],
        ),
        "walkdir": crate.spec(version = "2.3.3"),
        "anyhow": crate.spec(
            version = "1.0.68",
        ),
        "actix-web": crate.spec(
            version = "4.3.0",
        ),
        "actix": crate.spec(
            version = "0.13.0",
        ),
        "actix-identity": crate.spec(version = "0.6.0"),
        "actix-session": crate.spec(
            version = "0.8.0",
            features = ['cookie-session'],
        ),
        "actix-web-actors": crate.spec(
            version = "4.2.0",
        ),
        "actix-files": crate.spec(
            version = "0.6.2",
        ),
        "mime": crate.spec(version = "0.3.17"),
        "serde": crate.spec(
            version = "1.0.152",
        ),
        "bincode": crate.spec(version = "1.3.3"),
        "once_cell": crate.spec(version = "1.17.1"),

        # frontend
        "dotenvy_macro": crate.spec(version = "0.15.7",),
        "humantime": crate.spec(version = "2.1.0",),
        "js-sys": crate.spec(version = "0.3.66",), # tied to wasm-bindgen
        "obfstr": crate.spec(version = "0.4.3",),
        # "plotly": crate.spec(version = "0.8.3", features = ["wasm"],),
        "plotly": crate.spec(
            git = "https://github.com/mjpauly/plotly",
            branch = "all_new_features",
            features = ["wasm"],
        ),
        "serde_json": crate.spec(version = "1.0.94",),
        "serde-wasm-bindgen": crate.spec(version = "0.5.0",),
        "unicode-segmentation": crate.spec(version = "1.11.0"),
        "uom": crate.spec(
            version = "0.34.0",
            features = ["u64"],
        ),
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
            version = "0.3.36",
            features = ["local-offset", "wasm-bindgen", "formatting", "parsing",
                        "serde", "std",
            ],
        ),
        "futures": crate.spec(version = "0.3.26"),
        "gloo-net": crate.spec(version = "0.2.6"),
        "wasm-bindgen-futures": crate.spec(version = "0.4.39"), # tied to js-sys
        "getrandom": crate.spec( # dependency of uuid
            version = "0.2.8",
            features = ["js"]
        ),
        "uuid": crate.spec(
            version = "1.3.0",
            features = ["v4", "fast-rng", "macro-diagnostics",]
        ),
        "web-sys": crate.spec(  # tied to wasm-bindgen
            version = "0.3.66",
            features = [
                "Performance", "Window", "HtmlInputElement", "Location",
                "HtmlSelectElement", "Document", "Element", "CssStyleSheet",
                "StyleSheetList", "CssRuleList", "CssStyleDeclaration",
                "HtmlImageElement", "HtmlCanvasElement",
                "CanvasRenderingContext2d", "ImageData", "HtmlTextAreaElement",
            ],
        ),
        "log": crate.spec(
            version = "0.4.17",
            features = ["release_max_level_error"],
        ),
        "wasm-logger": crate.spec(
            version = "0.2.0",
        ),
        "yew_icons": crate.spec(
            version = "0.7.2",
            # git = "https://github.com/mjpauly/yew_icons",
            # branch = "main",
            features = [
                "BootstrapBoxArrowUp",
                "BootstrapBoxArrowUpRight",
                "BootstrapBrush",
                "BootstrapCalendarRange",
                "BootstrapCheck",
                "BootstrapCheckCircle",
                "BootstrapCheckCircleFill",
                "BootstrapChevronDown",
                "BootstrapChevronLeft",
                "BootstrapChevronRight",
                "BootstrapClipboard2Pulse",
                "BootstrapDownload",
                "BootstrapExclamationCircle",
                "BootstrapExclamationCircleFill",
                "BootstrapExclamationTriangle",
                "BootstrapExclamationTriangleFill",
                "BootstrapFullscreen",
                "BootstrapFunnel",
                "BootstrapGear",
                "BootstrapGlobeAmericas",
                "BootstrapGraphUp",
                "BootstrapInfoCircle",
                "BootstrapInfoCircleFill",
                "BootstrapJournalText",
                "BootstrapJoystick",
                "BootstrapList",
                "BootstrapPlus",
                "BootstrapPlusLg",
                "BootstrapQuestionCircle",
                "BootstrapSquare",
                "BootstrapSquareFill",
                "BootstrapTrash",
                "BootstrapX",
                "BootstrapXCircle",
                "BootstrapXCircleFill",
                "BootstrapXLg",
                "FontAwesomeSolidLocationArrow",
            ],
        ),
        # Tied to the wasm_bindgen version in rules_rust
        # Info: https://github.com/rustwasm/wasm-bindgen/issues/2776
        "wasm-bindgen": crate.spec(
            version = "=0.2.89",
        ),

        # dev + testing
        "tokio-tungstenite": crate.spec(version = "0.18.0",),
        "futures-util": crate.spec(version = "0.3.27",),
        "rusty-fork": crate.spec(version = "0.3.0",),
        "fantoccini": crate.spec(version = "0.19.3",),
        "pretty_assertions": crate.spec(version = "1.4.0"),
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

# crate index for the website, which has different dependency needs
crates_repository(
    name = "website_crate_index",
    cargo_lockfile = "//src/server/website:Cargo.lock",
    lockfile = "//src/server/website:Cargo.Bazel.lock",
    isolated = False,
    packages = {
        # Cross-compiling for OpenSSL is difficult without sysroot. Rustls is
        # stable so we use it instead.
        "actix-web": crate.spec( version = "4.3.0", features = ["rustls"]),
        "anyhow": crate.spec( version = "1.0.68",),
        "futures-util": crate.spec(version = "0.3.27",),
        "mime": crate.spec(version = "0.3.17"),
        "secrecy": crate.spec( version = "0.8.0",),
        "serde": crate.spec( version = "1.0.152",),
        "sqlx": crate.spec(
            version = "0.6.2",
            features = ["macros", "postgres", "runtime-tokio-rustls",
                        "time", "uuid"],
        ),
        "time": crate.spec( version = "0.3.20",),
        "tokio": crate.spec( version = "1.24.2", features = ["full"],),
        "tracing": crate.spec( version = "0.1.37",),
        "tracing-subscriber": crate.spec(
            version = "0.3.17",
            features = ["env-filter", "tracing-log"],
        ),
        "uuid": crate.spec(
            version = "1.3.0",
            features = ["v4", "fast-rng", "macro-diagnostics",]
        ),

        # dev + testing
        "reqwest": crate.spec(version = "0.11.15"),

    },
    splicing_config = splicing_config(resolver_version = "2"),
    render_config = render_config(
        default_package_name = ""
    ),
)

load("@website_crate_index//:defs.bzl", "crate_repositories")

crate_repositories()


# rust analyzer
# Our project isn't structured as a Cargo project, so we need to generate the
# rust-project.json for rust-analyzer to use.
# The rust-analyzer binary must be in your path.

load("@rules_rust//tools/rust_analyzer:deps.bzl", "rust_analyzer_dependencies")

rust_analyzer_dependencies()


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

# === Skylib === #

# provides convenience rules like select_file

http_archive(
    name = "bazel_skylib",
    sha256 = "b8a1527901774180afc798aeb28c4634bdccf19c4d98e7bdd1ce79d1fe9aaad7",
    urls = [
        "https://github.com/bazelbuild/bazel-skylib/releases/download/1.4.1/bazel-skylib-1.4.1.tar.gz",
    ],
)

load("@bazel_skylib//:workspace.bzl", "bazel_skylib_workspace")

bazel_skylib_workspace()



# === Zig C Compiler === #

HERMETIC_CC_TOOLCHAIN_VERSION = "v3.0.1"

http_archive(
    name = "hermetic_cc_toolchain",
    sha256 = "3bc6ec127622fdceb4129cb06b6f7ab098c4d539124dde96a6318e7c32a53f7a",
    urls = [
        "https://github.com/uber/hermetic_cc_toolchain/releases/download/{0}/hermetic_cc_toolchain-{0}.tar.gz".format(HERMETIC_CC_TOOLCHAIN_VERSION),
    ],
)

load("@hermetic_cc_toolchain//toolchain:defs.bzl", zig_toolchains = "toolchains")

# Plain zig_toolchains() will pick reasonable defaults. See
# toolchain/defs.bzl:toolchains on how to change the Zig SDK version and
# download URL.
zig_toolchains()


# === Rules Pkg === #

http_archive(
    name = "rules_pkg",
    urls = [
        "https://github.com/bazelbuild/rules_pkg/releases/download/0.9.1/rules_pkg-0.9.1.tar.gz",
    ],
    sha256 = "8f9ee2dc10c1ae514ee599a8b42ed99fa262b757058f65ad3c384289ff70c4b8",
)
load("@rules_pkg//:deps.bzl", "rules_pkg_dependencies")
rules_pkg_dependencies()


# === Docker Containers === #

# Go required for docker-less container operations
http_archive(
    name = "io_bazel_rules_go",
    sha256 = "6dc2da7ab4cf5d7bfc7c949776b1b7c733f05e56edc4bcd9022bb249d2e2a996",
    urls = [
        "https://github.com/bazelbuild/rules_go/releases/download/v0.39.1/rules_go-v0.39.1.zip",
    ],
)

# Gazelle required for container_push
http_archive(
    name = "bazel_gazelle",
    sha256 = "727f3e4edd96ea20c29e8c2ca9e8d2af724d8c7778e7923a854b2c80952bc405",
    urls = [
        "https://github.com/bazelbuild/bazel-gazelle/releases/download/v0.30.0/bazel-gazelle-v0.30.0.tar.gz",
    ],
)


load("@io_bazel_rules_go//go:deps.bzl", "go_register_toolchains", "go_rules_dependencies")
load("@bazel_gazelle//:deps.bzl", "gazelle_dependencies", "go_repository")

go_rules_dependencies()

go_register_toolchains(version = "1.20.5")

gazelle_dependencies()


# docker
http_archive(
    name = "io_bazel_rules_docker",
    sha256 = "b1e80761a8a8243d03ebca8845e9cc1ba6c82ce7c5179ce2b295cd36f7e394bf",
    urls = ["https://github.com/bazelbuild/rules_docker/releases/download/v0.25.0/rules_docker-v0.25.0.tar.gz"],
)

load(
    "@io_bazel_rules_docker//repositories:repositories.bzl",
    container_repositories = "repositories",
)

container_repositories()

load("@io_bazel_rules_docker//repositories:deps.bzl", container_deps = "deps")

container_deps()

load("@io_bazel_rules_docker//container:container.bzl", "container_pull")

container_pull(
    name = "busybox_base",
    architecture = "amd64",
    registry = "registry.hub.docker.com/library",
    repository = "busybox",
    tag = "1.36.1-glibc",
)


# === Android === #

http_archive(
    name = "platforms",
    urls = [
        "https://github.com/bazelbuild/platforms/releases/download/0.0.8/platforms-0.0.8.tar.gz",
    ],
    sha256 = "8150406605389ececb6da07cbcb509d5637a3ab9a24bc69b1101531367d89d74",
)

## Android

http_archive(
    name = "rules_android",
    sha256 = "cd06d15dd8bb59926e4d65f9003bfc20f9da4b2519985c27e190cddc8b7a7806",
    strip_prefix = "rules_android-0.1.1",
    urls = ["https://github.com/bazelbuild/rules_android/archive/v0.1.1.zip"],
)

load("@rules_android//android:rules.bzl", "android_sdk_repository")

android_sdk_repository(name = "androidsdk")

http_archive(
    name = "rules_android_ndk",
    sha256 = "b1a5ddd784e6ed915c2035c0db536a278b5f50c64412128c06877115991391ef",
    strip_prefix = "rules_android_ndk-877c68ef34c9f3353028bf490d269230c1990483",
    url = "https://github.com/bazelbuild/rules_android_ndk/archive/877c68ef34c9f3353028bf490d269230c1990483.zip",
)

load("@rules_android_ndk//:rules.bzl", "android_ndk_repository")

android_ndk_repository(name = "androidndk")

## Kotlin

http_archive(
    name = "rules_kotlin",
    sha256 = "15afe2d727f0dba572e0ce58f1dac20aec1441422ca65f7c3f7671b47fd483bf",
    url = "https://github.com/bazelbuild/rules_kotlin/releases/download/v1.7.0/rules_kotlin_release.tgz",
)

load("@rules_kotlin//kotlin:repositories.bzl", "kotlin_repositories", "kotlinc_version")

_KOTLIN_COMPILER_VERSION = "1.7.20"
_KOTLIN_COMPILER_SHA = "5e3c8d0f965410ff12e90d6f8dc5df2fc09fd595a684d514616851ce7e94ae7d"

kotlin_repositories(
    compiler_release = kotlinc_version(
        release = _KOTLIN_COMPILER_VERSION,
        sha256 = _KOTLIN_COMPILER_SHA,
    ),
)

load("@rules_kotlin//kotlin:core.bzl", "kt_register_toolchains")

kt_register_toolchains()

## JVM External

RULES_JVM_EXTERNAL_TAG = "4.5"
RULES_JVM_EXTERNAL_SHA = "b17d7388feb9bfa7f2fa09031b32707df529f26c91ab9e5d909eb1676badd9a6"

http_archive(
    name = "rules_jvm_external",
    strip_prefix = "rules_jvm_external-%s" % RULES_JVM_EXTERNAL_TAG,
    sha256 = RULES_JVM_EXTERNAL_SHA,
    url = "https://github.com/bazelbuild/rules_jvm_external/archive/%s.zip" % RULES_JVM_EXTERNAL_TAG,
)

load("@rules_jvm_external//:repositories.bzl", "rules_jvm_external_deps")

rules_jvm_external_deps()

load("@rules_jvm_external//:setup.bzl", "rules_jvm_external_setup")

rules_jvm_external_setup()

load("@rules_jvm_external//:defs.bzl", "maven_install")

_LIFECYCLE_VERSION = "2.7.0"

maven_install(
    artifacts = [
        "androidx.activity:activity:1.8.2",
        "androidx.annotation:annotation:1.7.1",
        "androidx.appcompat:appcompat:1.6.1",
        "androidx.core:core-ktx:1.12.0",
        "androidx.lifecycle:lifecycle-livedata-ktx:{}".format(_LIFECYCLE_VERSION),
        "androidx.lifecycle:lifecycle-process:{}".format(_LIFECYCLE_VERSION),
        "androidx.lifecycle:lifecycle-runtime-ktx:{}".format(_LIFECYCLE_VERSION),
        "androidx.lifecycle:lifecycle-service:{}".format(_LIFECYCLE_VERSION),
        "androidx.lifecycle:lifecycle-viewmodel-ktx:{}".format(_LIFECYCLE_VERSION),
        "androidx.lifecycle:lifecycle-viewmodel-savedstate:{}".format(_LIFECYCLE_VERSION),
    ],
    repositories = [
        "https://maven.google.com",
        "https://repo1.maven.org/maven2",
    ],
)

## Android app bundles

http_archive(
    name = "rules_android_app_bundles",
    sha256 = "07433c7349c6e6b4f31298886b19206593e75976c7ff916e96e772f3ce34b216",
    strip_prefix = "rules_android_app_bundles-0.1.1",
    url = "https://github.com/Bencodes/rules_android_app_bundles/archive/refs/tags/v0.1.1.tar.gz",
    # Patch removes the `android_binary` target from the macro, so we can define
    # it ourselves. Created with `git diff > out.patch`
    patches = ["//src/app/android:bundle/rules_bundle.patch"],
    patch_args = ["-p1"],
)

load("//src/app/android:bundle/bundle_deps.bzl", "download_app_bundle_dependencies")

download_app_bundle_dependencies()
