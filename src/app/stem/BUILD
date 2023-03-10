load("@rules_rust//rust:defs.bzl", "rust_library", "rust_binary", "rust_test")


rust_library(
    name = "stem",
    srcs = glob(["back/*.rs"]) + ["//src/app/stem/front:rs/common.rs"],
    edition = "2021",
    deps = [
        ":alloc",
        "@stem_crate_index//:sqlx",
        "@stem_crate_index//:tokio",
        "@stem_crate_index//:anyhow",
        "@stem_crate_index//:plotly",
        "@stem_crate_index//:actix-web",
        "@stem_crate_index//:actix",
        "@stem_crate_index//:actix-web-actors",
        "@stem_crate_index//:actix-files",
        "@stem_crate_index//:serde",
        "@stem_crate_index//:bincode",
        "@stem_crate_index//:zip",
    ],
)

# Unit tests embedded within the library.
rust_test(
    name = "unit_tests",
    crate = ":stem",
    size = "small",  # 20 MB RAM, 1 min timeout
)

# Integration tests of the library's public interface.
rust_test(
    name = "int_tests",
    srcs = glob(["tests/*.rs"]),
    edition = "2021",
    deps = [
        ":stem",
    ],
    visibility = ["//visibility:public"],
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
