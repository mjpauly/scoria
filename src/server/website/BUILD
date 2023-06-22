load("@rules_rust//rust:defs.bzl", "rust_library", "rust_binary", "rust_test")

# static files to bundle into the binary
static_file_targets = [
    ":index",
    ":tailwind",
    ":logo",
    ":favicon",
]

# env var definitions to tell rust compiler where to find the files
static_file_env_defs = {
    "INDEX_FILE": "$(location :index)",
    "TAILWIND_FILE": "$(location :tailwind)",
    "LOGO_FILE": "$(location :logo)",
    "FAVICON_FILE": "$(location :favicon)",
}

rust_binary(
    name = "website_bin",
    srcs = ["aft/main.rs"],
    edition = "2021",
    deps = [
        ":website",
        "@website_crate_index//:tokio",
    ],
)

rust_library(
    name = "website",
    srcs = glob(["aft/**/*.rs"]),
    compile_data = static_file_targets,
    rustc_env = static_file_env_defs,
    deps = [
        "@website_crate_index//:actix-web",
        "@website_crate_index//:mime",
        "@website_crate_index//:secrecy",
        "@website_crate_index//:serde",
        "@website_crate_index//:sqlx",
        "@website_crate_index//:time",
        "@website_crate_index//:tokio",
    ],
)

genrule(
    name = "tailwind",
    srcs = glob(["static/*.html"])
        + ["tailwind/tailwind.config.js", "tailwind/styles.css"],
    outs = ["tailwind.css"],
    cmd = """
cp $(location tailwind/tailwind.config.js) .
$(execpath @front_npm//tailwindcss/bin:tailwindcss) \
        -i $(location tailwind/styles.css) \
        -o $@
""",
    tools = ["@front_npm//tailwindcss/bin:tailwindcss"],
)

# static files

filegroup(
    name = "index",
    srcs = ["static/index.html"],
)

filegroup(
    name = "logo",
    srcs = ["static/epsilon.png"],
)

filegroup(
    name = "favicon",
    srcs = ["static/favicon.png"],
)

# filegroup(
    # name = "support",
    # srcs = ["static/support.html"],
# )

# filegroup(
    # name = "privacy",
    # srcs = ["static/privacy.html"],
# )
