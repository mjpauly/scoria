load("@rules_rust//rust:defs.bzl", "rust_library", "rust_binary", "rust_test")

rust_binary(
    name = "db_test",
    srcs = ["main.rs"],
    edition = "2021",
    data = ["//src/app/stem:dev_db"],
    deps = [
        "//src/app/stem",
        "//src/app/stem:common",
        "@stem_crate_index//:log",
        "@stem_crate_index//:sqlx",
        "@stem_crate_index//:time",
        "@stem_crate_index//:tokio",
    ],
)
