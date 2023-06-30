load("@rules_rust//rust:defs.bzl", "rust_library", "rust_binary", "rust_test")


# static files to bundle into the binary
static_file_targets = [
    "//src/app/stem/front:index",
    "//src/app/stem/front:wasm_file",
    "//src/app/stem/front:js_file",
    "//src/app/stem/front:tailwind",
    "//src/app/stem/front:plotlyjs",
    "//src/app/stem/front:maplibregl",
    "//src/app/stem/front:maplibregl_css",
    "//src/app/stem/front:when_in_use_auth",
    "//src/app/stem/front:always_auth",
]

# env var definitions to tell rust compiler where to find the files
static_file_env_defs = {
    "INDEX_FILE": "$(location //src/app/stem/front:index)",
    "WASM_FILE": "$(location //src/app/stem/front:wasm_file)",
    "JS_FILE": "$(location //src/app/stem/front:js_file)",
    "TAILWIND_FILE": "$(location //src/app/stem/front:tailwind)",
    "PLOTLY_FILE": "$(location //src/app/stem/front:plotlyjs)",
    "MAPLIBRE_FILE": "$(location //src/app/stem/front:maplibregl)",
    "MAPLIBRE_CSS": "$(location //src/app/stem/front:maplibregl_css)",
    "WHEN_IN_USE_AUTH_PNG": "$(location //src/app/stem/front:when_in_use_auth)",
    "ALWAYS_AUTH_PNG": "$(location //src/app/stem/front:always_auth)",
}

rust_library(
    name = "stem",
    srcs = glob(["back/*.rs"]),
    edition = "2021",
    compile_data =
        glob(["migrations/*.sql"])
        + [".env", "//:secrets"]
        + static_file_targets,
    rustc_env = static_file_env_defs,
    crate_features = select({
        "//:release": [],
        "//conditions:default": ["extra_debug_logging"],
    }),
    proc_macro_deps = [
        "@stem_crate_index//:dotenvy_macro",
    ],
    deps = [
        ":common",
        ":alloc",
        "@stem_crate_index//:rand",
        "@stem_crate_index//:sqlx",
        "@stem_crate_index//:tokio",
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
)

# definitions common to both the frontend and the backend
rust_library(
    name = "common",
    srcs = glob(["common/*.rs"]),
    edition = "2021",
    deps = [
        "@stem_crate_index//:serde",
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
        "@stem_crate_index//:env_logger",
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


# Objective c and c glue

# This being required is a bug in objc_library, ideally 'StemLib' could depend
# directly on 'stem'
cc_library(
    name = "stem_shim",
    deps = [":stem"],
)

# The objective C library we can import into Swift
objc_library(
    name = "StemLib",
    hdrs = ["StemLib.h"],
    module_name = "StemLib",
    visibility = ["//visibility:public"],
    deps = [":stem_shim"],
)

cc_library(
    name = "alloc",
    srcs = ["alloc.cc"],
)


# rustfmt.toml

exports_files(
    [
        "rustfmt.toml",
    ],
    visibility = ["//visibility:public"],
)

