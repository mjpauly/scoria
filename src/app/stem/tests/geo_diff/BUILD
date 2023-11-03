load("@rules_rust//rust:defs.bzl", "rust_test")

# Test that the 2d difference operation is well behaved on an input which is
# known to be problematic.
rust_test(
    name = "diff_test",
    srcs = ["main.rs"],
    edition = "2021",
    # data = [
        # "left.bin",
        # "right.bin",
    # ],
    deps = [
        "@stem_crate_index//:bincode",
        "@stem_crate_index//:geo",
        "@stem_crate_index//:geo-clipper",
    ],
)
