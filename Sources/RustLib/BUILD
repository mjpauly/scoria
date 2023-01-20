load("@rules_rust//rust:defs.bzl", "rust_library", "rust_test")

rust_library(
    name = "lib",
    srcs = glob(["src/*.rs"]),
    edition = "2021",
    deps = [
        ":alloc",
        "@crate_index_rustlib//:rand",
    ],
)

rust_test(
    name = "tests",
    srcs = glob(["tests/*.rs"]),
    edition = "2021",
    deps = [
        ":lib",
    ],
    visibility = ["//visibility:public"],
)


# objective c and c glue

# This being required is a bug in objc_library, ideally 'RustLib' could depend
# directly on 'lib'
cc_library(
    name = "lib_shim",
    deps = [":lib"],
)

objc_library(
    name = "RustLib",
    srcs = ["RustLib.m"],
    hdrs = ["RustLib.h"],
    module_name = "RustLib",
    visibility = ["//visibility:public"],
    deps = [":lib_shim"],
)

cc_library(
    name = "alloc",
    srcs = ["alloc.cc"],
)
