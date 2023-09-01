load("@rules_rust//rust:defs.bzl", "rust_library", "rust_binary", "rust_test")
load(":templates.bzl", "html_template", "md_template", "files")
load("//src/bzl:files.bzl", "files_and_envs")
load("@rules_pkg//pkg:tar.bzl", "pkg_tar")
load(
    "@io_bazel_rules_docker//container:container.bzl",
    "container_image",
    "container_push",
)

# See the README in this directory for more docs

# Push the container to our digitalocean registry
container_push(
    name = "push_amd64",
    image = ":image_amd64",
    format = "Docker",
    registry = "registry.digitalocean.com/epsln",
    repository = "site", # name of the image
    # tag = "latest", # latest is the default
)

container_image(
    name = "image_amd64",
    architecture = "amd64",
    # References container_pull from WORKSPACE (above)
    base = "@busybox_base//image",
    cmd = ["/app/website_bin"],
    enable_mtime_preservation = True,
    env = {
        "INSIDE_DOCKER": "true",
    },
    ports = ["8000"],
    tars = [":website_tar"],
    workdir = "/app",
)

pkg_tar(
    name = "website_tar",
    srcs = [":website_bin"],
    include_runfiles = True, # allows us to not compile static files into
                             # the binary (not used yet), see rules_docker
                             # README for more info.
    package_dir = "/app",
    stamp = 1,
)

# Static file targets to bundle into the binary, paired with the environment
# variable names that are passed to the compiler.
static_file_targets, static_file_envs = files_and_envs({
    ":index": "INDEX_FILE",
    ":privacy_policy": "PRIVACY_POLICY_FILE",
    ":first_release_post": "FIRST_RELEASE_POST",
    ":contact": "CONTACT_FILE",
    ":terms": "TERMS_FILE",
    ":tailwind": "TAILWIND_FILE",
    "static/scoria.png": "LOGO_FILE",
    "static/favicon.png": "FAVICON_FILE",
    ":template_top": "TEMPLATE_TOP_FILE",
    ":template_bottom": "TEMPLATE_BOTTOM_FILE",
    "static/screenshots/1.jpeg": "SCREENSHOT1",
    "static/screenshots/2.jpeg": "SCREENSHOT2",
    "static/screenshots/3.jpeg": "SCREENSHOT3",
    "static/screenshots/4.jpeg": "SCREENSHOT4",
    "static/Download_on_the_App_Store_Badge_US-UK_RGB_blk_092917.svg":
        "APP_STORE_BADGE",
})

rust_binary(
    name = "website_bin",
    srcs = ["aft/main.rs"],
    edition = "2021",
    deps = [
        ":website",
        "@website_crate_index//:tokio",
    ],
    # Extra optimizations on optimized builds
    # doc: https://doc.rust-lang.org/rustc/codegen-options/index.html
    rustc_flags = select({
        "//:optimized_build": [
            "-C", "strip=symbols", # strip symbols
            "-C", "lto=y", # link-time optimization
        ],
        "//conditions:default": [],
    }),
)

rust_library(
    name = "website",
    srcs = glob(["aft/**/*.rs"]),
    compile_data = static_file_targets + [".env"],
    rustc_env = static_file_envs,
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

# static file target definitions

genrule(
    name = "tailwind",
    srcs = glob(["static/*.html", "static/*.md"])
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

html_template(
    name = "index",
    src = "static/index.html",
)

md_template(
    name = "privacy_policy",
    src = "static/privacy_policy.md",
)

md_template(
    name = "first_release_post",
    src = "static/first_release.md",
    include_html = True,
)

html_template(
    name = "contact",
    src = "static/contact.html",
)

md_template(
    name = "terms",
    src = "static/terms.md",
)

# provides a convenient shorthand target name for the template files
files(
    names_srcs = {
        "template_top": "static/template_top.html",
        "template_bottom": "static/template_bottom.html",
    },
)
