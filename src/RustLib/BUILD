load("@rules_rust//rust:defs.bzl", "rust_library", "rust_test", "rust_doc")

rust_library(
    name = "lib",
    srcs = glob(["src/*.rs"]),
    edition = "2021",
    deps = [
        ":alloc",
        "@crate_index_rustlib//:rand",
    ],
)

# Unit tests embedded within the library.
rust_test(
    name = "unit_tests",
    crate = ":lib",
)

rust_doc(
    name = "lib_doc",
    crate = ":lib",
)

# Integration tests of the library's public interface.
rust_test(
    name = "int_tests",
    srcs = glob(["tests/*.rs"]),
    edition = "2021",
    deps = [
        ":lib",
    ],
    visibility = ["//visibility:public"],
)


# Objective c and c glue

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
