load("@rules_rust//rust:defs.bzl", "rust_library", "rust_binary")
load("@rules_rust//wasm_bindgen:wasm_bindgen.bzl", "rust_wasm_bindgen")
load("@rules_pkg//pkg:zip.bzl", "pkg_zip")

# all of the files we need for an http server to serve the frontend, bundled
# into a zip archive
pkg_zip(
    name = "dist",
    mode = "0666",  # read/write; default would be read/execute, which causes
                    # some downstream problems when trying to overwrite
    srcs = [
        ":front_wasm",
        ":tailwind",
        "index.html",
    ],
    visibility = ["//visibility:public"],
)

# top wasm app binary
rust_wasm_bindgen(
    name = "front_wasm",
    target = "web",
    wasm_file = ":front_app",
)

# binary that runs the app
rust_binary(
    name = "front_app",
    srcs = glob(["rs/main.rs"]),
    edition = "2021",
    deps = [
        ":front",
        "@rules_rust//wasm_bindgen/3rdparty:wasm_bindgen",
        "@stem_crate_index//:yew",
        "@stem_crate_index//:log",
        "@stem_crate_index//:wasm-logger",
    ],
)

# library of the app that gets run by :front_app
rust_library(
    name = "front",
    srcs = glob(["rs/**/*.rs"]),
    edition = "2021",
    deps = [
        "@stem_crate_index//:yew",
        "@stem_crate_index//:yew-router",
        "@stem_crate_index//:web-sys",
        "@stem_crate_index//:log",
        "@stem_crate_index//:wasm-logger",
        "@stem_crate_index//:yew_icons",
    ],
)

genrule(
    name = "tailwind",
    srcs = glob(["rs/**/*.rs"]) + ["tailwind.config.js", "styles.css"],
    outs = ["tailwind.css"],
    cmd = """
cp $(location tailwind.config.js) .
$(execpath @front_npm//tailwindcss/bin:tailwindcss) -i $(location styles.css) \
        -o $@
""",
    tools = ["@front_npm//tailwindcss/bin:tailwindcss"],
)

