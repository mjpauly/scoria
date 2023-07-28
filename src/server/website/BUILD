load("@rules_rust//rust:defs.bzl", "rust_library", "rust_binary", "rust_test")
load(":templates.bzl", "html_template", "md_template", "files")
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
static_files = {
    ":index": "INDEX_FILE",
    ":privacy_policy": "PRIVACY_POLICY_FILE",
    ":first_release_blog": "FIRST_RELEASE_BLOG",
    ":contact": "CONTACT_FILE",
    ":terms": "TERMS_FILE",
    ":tailwind": "TAILWIND_FILE",
    ":logo": "LOGO_FILE",
    ":favicon": "FAVICON_FILE",
    ":template_top": "TEMPLATE_TOP_FILE",
    ":template_bottom": "TEMPLATE_BOTTOM_FILE",
    ":screenshot1": "SCREENSHOT1",
    ":screenshot2": "SCREENSHOT2",
    ":screenshot3": "SCREENSHOT3",
    ":screenshot4": "SCREENSHOT4",
    ":app_store_badge": "APP_STORE_BADGE",
}

# List of the static file target names
static_file_targets = static_files.keys()

# Dict of the environment variable names paired with the target location.
# The location expresison is evaluated by Bazel before passing to the compiler.
static_file_env_defs = {envar: "$(location %s)" % target for target, envar in static_files.items()}


rust_binary(
    name = "website_bin",
    srcs = ["aft/main.rs"],
    edition = "2021",
    deps = [
        ":website",
        "@website_crate_index//:tokio",
    ],
    # Strip debug symbols (-s) on optimized builds when linking.
    # doc: https://www.mankier.com/1/ld.lld
    rustc_flags = select({
        "//:optimized_build": ["-C", "link-args=-s"],
        "//conditions:default": [],
    }),
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
    name = "first_release_blog",
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

files(
    names_srcs = {
        "template_top": "static/template_top.html",
        "template_bottom": "static/template_bottom.html",
        "logo": "static/epsilon.png",
        "favicon": "static/favicon.png",
        "screenshot1": "static/screenshots/1.jpeg",
        "screenshot2": "static/screenshots/2.jpeg",
        "screenshot3": "static/screenshots/3.jpeg",
        "screenshot4": "static/screenshots/4.jpeg",
        "app_store_badge": "static/Download_on_the_App_Store_Badge_US-UK_RGB_blk_092917.svg",
    },
)
