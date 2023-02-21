load("@rules_rust//rust:defs.bzl", "rust_library", "rust_test", "rust_doc")

rust_library(
    name = "stem",
    srcs = glob(["pith/*.rs"]),
    edition = "2021",
    deps = [
        ":alloc",
        "@stem_crate_index//:sqlx",
        "@stem_crate_index//:tokio",
        "@stem_crate_index//:anyhow",
        "@stem_crate_index//:plotly",
    ],
)

# Unit tests embedded within the library.
rust_test(
    name = "unit_tests",
    crate = ":stem",
    size = "small",  # 20 MB RAM, 1 min timeout
)

rust_doc(
    name = "stem_doc",
    crate = ":stem",
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

# testing bundling files into the app
filegroup(
    name = "libfiles",
    srcs = [
        "filetobundle.txt",
    ],
    visibility = ["//visibility:public"],
)
