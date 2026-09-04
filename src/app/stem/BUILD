load("@rules_rust//rust:defs.bzl", "rust_library", "rust_binary", "rust_test")
load("//src/bzl:files.bzl", "files_and_envs")


# static files to bundle into the binary
static_file_targets, static_file_envs = files_and_envs({
    "//src/app/stem/front:index.html": "INDEX_FILE",
    "//src/app/stem/front:wasm_file": "WASM_FILE",
    "//src/app/stem/front:js_file": "JS_FILE",
    "//src/app/stem/front:tailwind": "TAILWIND_FILE",
    "//src/app/stem/front:static/plotly.min.js": "PLOTLY_FILE",
    "//src/app/stem/front:static/maplibre-gl.js": "MAPLIBRE_FILE",
    "//src/app/stem/front:static/maplibre-gl.css": "MAPLIBRE_CSS",
    "//src/app/stem/front:static/when_in_use_auth_ios.png": "WHEN_IN_USE_AUTH_IOS_PNG",
    "//src/app/stem/front:static/when_in_use_auth_android.png": "WHEN_IN_USE_AUTH_ANDROID_PNG",
    "//src/app/stem/front:static/always_auth.png": "ALWAYS_AUTH_PNG",
    "//src/app/stem/front:static/blank.png": "BLANK_PNG",
})

platform_features = select({
    "//:android_config": ["android_config"],
    "//conditions:default": ["ios_config"],
})

rust_library(
    name = "stem",
    srcs = glob(["back/**/*.rs"]),
    edition = "2021",
    compile_data =
        glob(["migrations/*.sql"])
        + [".env", "//:secrets"]
        + select({
            # embed static files if not autoreloading
            "//:autoreload_on": [],
            "//conditions:default": static_file_targets,
        }),
    rustc_env = select({
        # embed static files if not autoreloading
        "//:autoreload_on": {},
        "//conditions:default": static_file_envs,
    }),
    rustc_flags = select({ # extra optimizations (stripping must be done later)
        "//:optimized_build": ["-C", "lto=y"], # link-time optimization
        "//conditions:default": [],
    }),
    crate_features = platform_features + select({
        "//:distribution_profile": ["distribution_key"],
        "//conditions:default": [],
    }) + select({
        # compile support for runfiles instead of embedding the static files
        "//:autoreload_on": ["runfiles"],
        "//conditions:default": [],
    }),
    proc_macro_deps = [
        "@stem_crate_index//:dotenvy_macro",
    ],
    deps = [
        ":common",
        ":alloc",
        "@stem_crate_index//:base64",
        "@stem_crate_index//:bon",
        "@stem_crate_index//:csv",
        "@stem_crate_index//:futures-core",
        "@stem_crate_index//:geo",
        "@stem_crate_index//:geo-clipper",
        "@stem_crate_index//:geojson",
        "@stem_crate_index//:gpx",
        "@stem_crate_index//:itertools",
        "@stem_crate_index//:jiff",
        "@stem_crate_index//:log-panics",
        "@stem_crate_index//:mvt",
        "@stem_crate_index//:nav-types",
        "@stem_crate_index//:obfstr",
        "@stem_crate_index//:pointy",
        "@stem_crate_index//:rand",
        "@stem_crate_index//:reqwest",
        "@stem_crate_index//:sqlx",
        "@stem_crate_index//:tracing",
        "@stem_crate_index//:tracing-appender",
        "@stem_crate_index//:tracing-subscriber",
        "@stem_crate_index//:tzf-rs",
        "@stem_crate_index//:tokio",
        "@stem_crate_index//:walkdir",
        "@stem_crate_index//:anyhow",
        "@stem_crate_index//:actix-web",
        "@stem_crate_index//:actix",
        "@stem_crate_index//:actix-identity",
        "@stem_crate_index//:actix-session",
        "@stem_crate_index//:actix-web-actors",
        "@stem_crate_index//:actix-files",
        "@stem_crate_index//:mime",
        "@stem_crate_index//:serde",
        "@stem_crate_index//:serde_json",
        "@stem_crate_index//:bincode",
        "@stem_crate_index//:time",
        "@stem_crate_index//:once_cell",
        "@stem_crate_index//:uuid",
    ] + select({
        "//:android_config": [
            "@stem_crate_index//:android-tzdata",
            "@stem_crate_index//:iana-time-zone",
        ],
        "//conditions:default": [],
    }),
    visibility = [":__subpackages__"],
)

# Unit tests embedded within the stem crate.
rust_test(
    name = "unit_tests",
    crate = ":stem",
    size = "small",  # 20 MB RAM, 1 min timeout
    crate_features = platform_features,
    deps = [
        "@stem_crate_index//:approx",
        "@stem_crate_index//:nalgebra",
        "@stem_crate_index//:reqwest", # for server health check unit test
        "@stem_crate_index//:futures-core",
        "@stem_crate_index//:pretty_assertions",
    ]
)

# definitions common to both the frontend and the backend
rust_library(
    name = "common",
    srcs = glob(["common/**/*.rs"]),
    edition = "2021",
    deps = [
        "@stem_crate_index//:anyhow",
        "@stem_crate_index//:jiff",
        "@stem_crate_index//:serde",
        "@stem_crate_index//:strum",
        "@stem_crate_index//:time",
        "@stem_crate_index//:unicode-segmentation",
        "@stem_crate_index//:uom",
        "@stem_crate_index//:uuid",
        "@stem_crate_index//:serde_json",
        "@stem_crate_index//:serde_qs",
    ],
    visibility = ["//visibility:public"],
)

# Unit tests for the common crate.
rust_test(
    name = "common_tests",
    crate = ":common",
    size = "small",
    deps = [
        "@stem_crate_index//:pretty_assertions",
    ]
)

# Integration tests of the library's public interface.
rust_test(
    name = "int_tests",
    srcs = glob(["tests/*.rs"]),
    # crate_features = platform_features,
    deps = [
        ":stem",
        ":common",
        "@stem_crate_index//:bincode",
        "@stem_crate_index//:fantoccini",
        "@stem_crate_index//:futures-util",
        "@stem_crate_index//:jiff",
        "@stem_crate_index//:rusty-fork",
        "@stem_crate_index//:time",
        "@stem_crate_index//:tokio",
        "@stem_crate_index//:tokio-tungstenite",
    ],
    visibility = ["//visibility:public"],
)

# Stem runner for development of the frontend and backend. Using ibazel allows
# for hot reloading
rust_binary(
    name = "dev",
    srcs = [
        "dev/main.rs",
        "dev/synth.rs",
    ],
    edition = "2021",
    data = [
        "db/data.db",
        # "db_full/saved_places.json",
        # "db_full/import_seg.db",
    ] + select({
        # make the static files available as runfiles
        "//:autoreload_on": static_file_targets,
        "//conditions:default": [],
    }),
    deps = [
        ":stem",
        ":common",
        "@stem_crate_index//:jiff",
        "@stem_crate_index//:tokio",
        "@stem_crate_index//:log",
        "@stem_crate_index//:time",
        "@stem_crate_index//:rand",
    ],
)

# Memory probe harness: measures browser-side memory per byte of geojson via
# headless Chromium over CDP. See doc/decimation/memory-limits.md, Phase 2.
# Run with: bazel run //src/app/stem:memory_probe -- --help
sh_binary(
    name = "memory_probe",
    srcs = ["dev/memory_probe.sh"],
)

# Development database generator for sqlx query macros. Database is placed in
# db/. Also creates a .env file in stem/ to tell sqlx where to find the
# development database. Run `bazel run :db_gen`
rust_binary(
    name = "db_gen",
    srcs = ["dev/db_gen.rs"],
    edition = "2021",
    compile_data = glob(["migrations/*.sql"]),
    deps = [
        "@stem_crate_index//:tokio",
        "@stem_crate_index//:sqlx",
    ],
)


# C interface exposed for interop with Swift and Java
cc_library(
    name = "stem_cc",
    hdrs = ["stem.h"],
    deps = [":stem"],
    visibility = ["//visibility:public"],
)

cc_library(
    name = "alloc",
    srcs = ["alloc.cc"],
)


# Exports to subpackages

# dev database, useful for querying against real data
filegroup(
    name = "dev_db",
    srcs = glob(["db/data.db*"]),
    visibility = [":__subpackages__"],
)
