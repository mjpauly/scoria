load("@rules_rust//rust:defs.bzl", "rust_library", "rust_binary", "rust_test")

# static files to bundle into the binary
static_file_targets = [
    ":index",
    ":privacy_policy",
    ":contact",
    ":tailwind",
    ":logo",
    ":favicon",
    ":template_top",
    ":template_bottom",
]

# env var definitions to tell rust compiler where to find the files
static_file_env_defs = {
    "INDEX_FILE": "$(location :index)",
    "PRIVACY_POLICY_FILE": "$(location :privacy_policy)",
    "CONTACT_FILE": "$(location :contact)",
    "TAILWIND_FILE": "$(location :tailwind)",
    "LOGO_FILE": "$(location :logo)",
    "FAVICON_FILE": "$(location :favicon)",
    "TEMPLATE_TOP_FILE": "$(location :template_top)",
    "TEMPLATE_BOTTOM_FILE": "$(location :template_bottom)",
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
    compile_data = static_file_targets + [".env"],
    rustc_env = static_file_env_defs,
    deps = [
        "@website_crate_index//:actix-web",
        "@website_crate_index//:anyhow",
        "@website_crate_index//:mime",
        "@website_crate_index//:secrecy",
        "@website_crate_index//:serde",
        "@website_crate_index//:sqlx",
        "@website_crate_index//:time",
        "@website_crate_index//:tokio",
        "@website_crate_index//:uuid",
    ],
)

# static files

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

genrule(
    name = "privacy_policy",
    srcs = [ "static/privacy_policy.md", ":template_top", ":template_bottom" ],
    outs = ["privacy_policy.html"],
    cmd = """
cmark $(location static/privacy_policy.md) -t html > content.html

cat $(location :template_top) >> $@
cat content.html >> $@
cat $(location :template_bottom) >> $@
""",
)

genrule(
    name = "contact",
    srcs = ["static/contact.html", ":template_top", ":template_bottom"],
    outs = ["contact.html"],
    cmd = """
cat $(location :template_top) >> $@
cat $(location static/contact.html) >> $@
cat $(location :template_bottom) >> $@
""",
)

genrule(
    name = "index",
    srcs = ["static/index.html", ":template_top", ":template_bottom"],
    outs = ["index.html"],
    cmd = """
cat $(location :template_top) >> $@
cat $(location static/index.html) >> $@
cat $(location :template_bottom) >> $@
""",
)

filegroup(
    name = "template_top",
    srcs = ["static/template_top.html"],
)

filegroup(
    name = "template_bottom",
    srcs = ["static/template_bottom.html"],
)

filegroup(
    name = "logo",
    srcs = ["static/epsilon.png"],
)

filegroup(
    name = "favicon",
    srcs = ["static/favicon.png"],
)
