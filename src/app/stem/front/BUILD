load("@rules_rust//rust:defs.bzl", "rust_library", "rust_binary", "rust_doc")
load("@rules_rust//wasm_bindgen:defs.bzl", "rust_wasm_bindgen")
load("@bazel_skylib//rules:select_file.bzl", "select_file")

# The generated files to serve (wasm, js launcher)
select_file(
    name = "wasm_file",
    srcs = ":front_wasm",
    subpath = "front_wasm_bg.wasm",
    visibility = ["//visibility:public"],
)
select_file(
    name = "js_file",
    srcs = ":front_wasm",
    subpath = "front_wasm.js",
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
    # crate features used to select desired api keys
    crate_features = select({
        "//:autoreload_on": ["dev_autoreload"],
        "//conditions:default": [],
    }) + select({
        "//:android_config": ["android_config"],
        "//conditions:default": ["ios_config"],
    }),
    proc_macro_deps = [
        "@stem_crate_index//:dotenvy_macro",
    ],
    deps = [
        "//src/app/stem:common",
        "@stem_crate_index//:bincode",
        "@stem_crate_index//:futures",
        "@stem_crate_index//:gloo-net",
        "@stem_crate_index//:humantime",
        "@stem_crate_index//:itertools",
        "@stem_crate_index//:js-sys",
        "@stem_crate_index//:log",
        "@stem_crate_index//:obfstr",
        "@stem_crate_index//:plotly",
        "@stem_crate_index//:serde",
        "@stem_crate_index//:serde-wasm-bindgen",
        "@stem_crate_index//:serde_json",
        "@stem_crate_index//:strum",
        "@stem_crate_index//:time",
        "@stem_crate_index//:unicode-segmentation",
        "@stem_crate_index//:uom",
        "@stem_crate_index//:url",
        "@stem_crate_index//:uuid",
        "@stem_crate_index//:wasm-bindgen",
        "@stem_crate_index//:wasm-bindgen-futures",
        "@stem_crate_index//:wasm-logger",
        "@stem_crate_index//:web-sys",
        "@stem_crate_index//:yew",
        "@stem_crate_index//:yew-hooks",
        "@stem_crate_index//:yew-router",
        "@stem_crate_index//:yew_icons",
        "@stem_crate_index//:yewdux",
    ],
)

# generated documentation for the frontend
rust_doc(
    name = "front_doc",
    crate = ":front",
    rustdoc_flags = ["--document-private-items",],
)

genrule(
    name = "tailwind",
    srcs = glob(["rs/**/*.rs"]) + ["tailwind.config.js", "styles.css"],
    outs = ["tailwind.css"],
    cmd = """
cp $(location tailwind.config.js) .

# We have to tell node where to find the plugin and, since the plugin imports
# from tailwind, where to find tailwind's code as well. We point node to each
# of these by adding the node_modules directories to the NODE_PATH envar.
# For the plugin, we query the plugin's location, then use dirname to go up one
# level for the node_modules directory.
# For tailwind, we append the binary path with the right runfiles directory
export NODE_PATH=$$(dirname $(location @front_npm//:node_modules/tailwindcss-safe-area)):$(location @front_npm//tailwindcss/bin:tailwindcss).runfiles/front_npm/node_modules

$(execpath @front_npm//tailwindcss/bin:tailwindcss) -i $(location styles.css) \
        -o $@
""",
    tools = [
        "@front_npm//tailwindcss/bin:tailwindcss",
        "@front_npm//:node_modules/tailwindcss-safe-area",
    ],
    visibility = ["//visibility:public"],
)

# Script to download the plotly and maplibre libs and place them in the static/
# directory. This persists the libraries longer than bazel's cache might,
# ensuring they are accessible for the lifetime of this this work tree.
# Run the download with `bazel run :download_third_party_js_libraries`
sh_binary(
    name = "download_js_libs",
    srcs = ["download_js_libs.sh"],
)

exports_files(
    [
        "index.html",
        "static/plotly.min.js",
        "static/maplibre-gl.js",
        "static/maplibre-gl.css",

        # images showing what location authorizations look like
        "static/when_in_use_auth_ios.png",
        "static/when_in_use_auth_android.png",
        "static/always_auth.png",

        # blank image to show on map if raster data is not to be shown
        "static/blank.png",
    ],
    visibility = ["//src/app/stem:__pkg__"],
)
