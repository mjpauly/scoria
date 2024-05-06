load("@rules_rust//rust:defs.bzl", "rust_binary")

rust_binary(
    name = "emu", # Can't be the same as the name as the subdirectory
    srcs = ["emu_geo/main.rs"],
    edition = "2021",
    data = ["emu_geo/test_track.gpx"],
    deps = [
        "@stem_crate_index//:tokio",
        "@stem_crate_index//:gpx",
    ],
)
