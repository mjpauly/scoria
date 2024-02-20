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
    "//src/app/stem/front:static/when_in_use_auth.png": "WHEN_IN_USE_AUTH_PNG",
    "//src/app/stem/front:static/always_auth.png": "ALWAYS_AUTH_PNG",
    "//src/app/stem/front:static/blank.png": "BLANK_PNG",
})

rust_library(
    name = "stem",
    srcs = glob(["back/**/*.rs"]),
    edition = "2021",
    compile_data =
        glob(["migrations/*.sql"])
        + [".env", "//:secrets"]
        + static_file_targets,
    rustc_env = static_file_envs,
    rustc_flags = select({ # extra optimizations (stripping must be done later)
        "//:optimized_build": ["-C", "lto=y"], # link-time optimization
        "//conditions:default": [],
    }),
    crate_features = select({
        "//:distribution_profile": [],
        "//conditions:default": ["extra_debug_logging"],
    }),
    proc_macro_deps = [
        "@stem_crate_index//:dotenvy_macro",
    ],
    deps = [
        ":common",
        ":alloc",
        "@stem_crate_index//:csv",
        "@stem_crate_index//:geo",
        "@stem_crate_index//:geo-clipper",
        "@stem_crate_index//:geojson",
        "@stem_crate_index//:gpx",
        "@stem_crate_index//:itertools",
        "@stem_crate_index//:log-panics",
        "@stem_crate_index//:mvt",
        "@stem_crate_index//:obfstr",
        "@stem_crate_index//:pointy",
        "@stem_crate_index//:rand",
        "@stem_crate_index//:reqwest",
        "@stem_crate_index//:sqlx",
        "@stem_crate_index//:tracing",
        "@stem_crate_index//:tracing-appender",
        "@stem_crate_index//:tracing-subscriber",
        "@stem_crate_index//:tokio",
        "@stem_crate_index//:walkdir",
        "@stem_crate_index//:anyhow",
        "@stem_crate_index//:actix-web",
        "@stem_crate_index//:actix",
        "@stem_crate_index//:actix-web-actors",
        "@stem_crate_index//:actix-files",
        "@stem_crate_index//:mime",
        "@stem_crate_index//:serde",
        "@stem_crate_index//:serde_json",
        "@stem_crate_index//:bincode",
        "@stem_crate_index//:time",
        "@stem_crate_index//:once_cell",
    ],
    visibility = [":__subpackages__"],
)

# definitions common to both the frontend and the backend
rust_library(
    name = "common",
    srcs = glob(["common/*.rs"]),
    edition = "2021",
    deps = [
        "@stem_crate_index//:serde",
        "@stem_crate_index//:strum",
        "@stem_crate_index//:time",
        "@stem_crate_index//:uom",
    ],
    visibility = ["//visibility:public"],
)

# Unit tests embedded within the library.
rust_test(
    name = "unit_tests",
    crate = ":stem",
    size = "small",  # 20 MB RAM, 1 min timeout
    crate_features = ["extra_debug_logging"],
    deps = [
        "@stem_crate_index//:reqwest", # for server health check unit test
        "@stem_crate_index//:futures-core",
    ]
)

# Integration tests of the library's public interface.
rust_test(
    name = "int_tests",
    srcs = glob(["tests/*.rs"]),
    deps = [
        ":stem",
        ":common",
        "@stem_crate_index//:bincode",
        "@stem_crate_index//:fantoccini",
        "@stem_crate_index//:futures-util",
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
    srcs = ["dev/main.rs"],
    edition = "2021",
    data = [
        "db/data.db",
    ],
    deps = [
        ":stem",
        "@stem_crate_index//:tokio",
        "@stem_crate_index//:log",
        "@stem_crate_index//:time",
        "@stem_crate_index//:rand",
    ],
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
