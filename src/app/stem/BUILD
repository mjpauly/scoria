load("@rules_rust//rust:defs.bzl", "rust_library", "rust_binary", "rust_test")


rust_library(
    name = "stem",
    srcs = glob(["back/*.rs"]) + ["//src/app/stem/front:rs/common.rs"],
    edition = "2021",
    compile_data = glob(["migrations/*.sql"]) + [".env"],
    deps = [
        ":alloc",
        "@stem_crate_index//:sqlx",
        "@stem_crate_index//:tokio",
        "@stem_crate_index//:anyhow",
        "@stem_crate_index//:actix-web",
        "@stem_crate_index//:actix",
        "@stem_crate_index//:actix-web-actors",
        "@stem_crate_index//:actix-files",
        "@stem_crate_index//:serde",
        "@stem_crate_index//:bincode",
        "@stem_crate_index//:zip",
        "@stem_crate_index//:time",
        "@stem_crate_index//:once_cell",
    ],
)

# Unit tests embedded within the library.
rust_test(
    name = "unit_tests",
    crate = ":stem",
    size = "small",  # 20 MB RAM, 1 min timeout
    data = ["//src/app/stem/front:dist"],
    deps = [
        "@stem_crate_index//:reqwest", # for server health check unit test
    ]
)

# Integration tests of the library's public interface.
rust_test(
    name = "int_tests",
    srcs = glob(["tests/*.rs"]),
    data = ["//src/app/stem/front:dist"],
    deps = [
        ":stem",
        "@stem_crate_index//:bincode",
        "@stem_crate_index//:fantoccini",
        "@stem_crate_index//:futures-util",
        "@stem_crate_index//:rusty-fork",
        "@stem_crate_index//:time",
        "@stem_crate_index//:tokio",
        "@stem_crate_index//:tokio-tungstenite",
    ],
)

# Stem runner for development of the frontend and backend. Using ibazel allows
# for hot reloading
rust_binary(
    name = "dev",
    srcs = ["dev/main.rs"],
    edition = "2021",
    data = ["//src/app/stem/front:dist"],
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
# dev/db/. Also creates a .env file in stem/ to tell sqlx where to find the
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

