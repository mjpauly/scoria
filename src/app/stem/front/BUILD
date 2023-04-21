load("@rules_rust//rust:defs.bzl", "rust_library", "rust_binary", "rust_doc")
load("@rules_rust//wasm_bindgen:wasm_bindgen.bzl", "rust_wasm_bindgen")
load("@rules_pkg//pkg:zip.bzl", "pkg_zip")

exports_files(
    ["rs/common.rs"],
    visibility = ["//visibility:public"],
)

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
        ":plotlyjs",
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
    compile_data = ["//:secrets"],
    proc_macro_deps = [
        "@stem_crate_index//:dotenvy_macro",
    ],
    deps = [
        "@stem_crate_index//:humantime",
        "@stem_crate_index//:js-sys",
        "@stem_crate_index//:obfstr",
        "@stem_crate_index//:plotly",
        "@stem_crate_index//:serde_json",
        "@stem_crate_index//:yew",
        "@stem_crate_index//:yew-hooks",
        "@stem_crate_index//:yew-router",
        "@stem_crate_index//:yewdux",
        "@stem_crate_index//:time",
        "@stem_crate_index//:futures",
        "@stem_crate_index//:gloo-net",
        "@stem_crate_index//:wasm-bindgen",
        "@stem_crate_index//:wasm-bindgen-futures",
        "@stem_crate_index//:web-sys",
        "@stem_crate_index//:log",
        "@stem_crate_index//:wasm-logger",
        "@stem_crate_index//:yew_icons",
        "@stem_crate_index//:serde",
        "@stem_crate_index//:bincode",
        "@stem_crate_index//:uuid",
    ],
)

# generated documentation for the frontend
rust_doc(
    name = "front_doc",
    crate = ":front",
    rustc_flags = ["--document-private-items",],
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

# download plotly to include with static files
genrule(
    name = "plotlyjs",
    outs = ["plotly.min.js"],
    cmd = 'curl https://cdn.plot.ly/plotly-2.20.0.min.js -o $@',
)
