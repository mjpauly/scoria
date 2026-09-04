load("@rules_rust//rust:defs.bzl", "rust_library", "rust_binary", "rust_test")
load(":templates.bzl", "html_template", "md_template", "files")
load("//src/bzl:files.bzl", "files_and_envs")
load("@rules_pkg//pkg:tar.bzl", "pkg_tar")
load("@rules_oci//oci:defs.bzl", "oci_image", "oci_push", "oci_tarball")
load("//src/bzl:platforms.bzl", "linux_amd64_files")

# See the README in this directory for more docs

# Push the container to our digitalocean registry
oci_push(
    name = "push_amd64",
    image = ":image_amd64",
    repository = "registry.digitalocean.com/epsln/site",
    remote_tags = ["latest"],
    tags = ["manual"],
)

# Load the container into the local docker daemon
oci_tarball(
    name = "load_amd64",
    image = ":image_amd64",
    repo_tags = ["epsln/site:latest"],
    tags = ["manual"],
)

# The image is always cross-compiled for linux/amd64, while push and load
# above run under the host platform so their tools (crane, yq) resolve for
# the host. Passing --platforms on the command line would break them.
linux_amd64_files(
    name = "image_amd64",
    srcs = [":website_image"],
    tags = ["manual"],
)

oci_image(
    name = "website_image",
    # References oci_pull from WORKSPACE
    base = "@busybox_base",
    cmd = ["/app/website_bin"],
    env = {
        "INSIDE_DOCKER": "true",
    },
    exposed_ports = ["8000"],
    tars = [":website_tar"],
    workdir = "/app",
    tags = ["manual"],
)

pkg_tar(
    name = "website_tar",
    srcs = [":website_bin"],
    include_runfiles = True, # allows us to not compile static files into
                             # the binary (not used yet).
    package_dir = "/app",
    stamp = 1,
)

# Static file targets to bundle into the binary, paired with the environment
# variable names that are passed to the compiler.
static_file_targets, static_file_envs = files_and_envs({
    ":index": "INDEX_FILE",
    ":privacy_policy": "PRIVACY_POLICY_FILE",
    ":feed": "FEED_FILE",
    ":posts": "POSTS_FILE",
    ":tips_for_use_post": "TIPS_FOR_USE_POST",
    ":import_places_post": "IMPORT_PLACES_POST",
    ":1_4_0_release_post": "1_4_0_RELEASE_POST",
    ":android_release_post": "ANDROID_RELEASE_POST",
    ":first_release_post": "FIRST_RELEASE_POST",
    ":contact": "CONTACT_FILE",
    ":terms": "TERMS_FILE",
    ":unavailable": "UNAVAILABLE_FILE",
    ":android_release_notes": "ANDROID_RELEASE_NOTES_FILE",
    ":android_notes": "ANDROID_NOTES_FILE",
    ":tailwind": "TAILWIND_FILE",
    "static/scoria.png": "LOGO_FILE",
    "static/favicon.png": "FAVICON_FILE",
    ":template_top": "TEMPLATE_TOP_FILE",
    ":template_bottom": "TEMPLATE_BOTTOM_FILE",
    "static/screenshots/1.jpeg": "SCREENSHOT1",
    "static/screenshots/2.jpeg": "SCREENSHOT2",
    "static/screenshots/3.jpeg": "SCREENSHOT3",
    "static/screenshots/4.jpeg": "SCREENSHOT4",
    "static/screenshots/5.jpeg": "SCREENSHOT5",
    "static/screenshots/6.jpeg": "SCREENSHOT6",
    "static/screenshots/7.jpeg": "SCREENSHOT7",
    "static/Download_on_the_App_Store_Badge_US-UK_RGB_blk_092917.svg":
        "APP_STORE_BADGE",
    "static/apk/Scoria_1.3.0.apk": "SCORIA_1_3_0",
    "static/apk/Scoria_1.4.0.apk": "SCORIA_1_4_0",
    "static/apk/Scoria_1.4.2.apk": "SCORIA_1_4_2",
    "static/apk/Scoria_1.4.3.apk": "SCORIA_1_4_3",
    "static/apk/Scoria_1.4.4.apk": "SCORIA_1_4_4",
    "static/apk/Scoria_1.5.0.apk": "SCORIA_1_5_0",
})

rust_binary(
    name = "website_bin",
    srcs = ["aft/main.rs"],
    edition = "2021",
    deps = [
        ":website",
        "@website_crate_index//:tokio",
        "@website_crate_index//:tracing",
        "@website_crate_index//:tracing-subscriber",
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
        "@website_crate_index//:futures-util",
        "@website_crate_index//:mime",
        "@website_crate_index//:secrecy",
        "@website_crate_index//:serde",
        "@website_crate_index//:sqlx",
        "@website_crate_index//:time",
        "@website_crate_index//:tokio",
        "@website_crate_index//:tracing",
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
    name = "posts",
    src = "static/posts.md",
)

md_template(
    name = "tips_for_use_post",
    src = "static/posts/tips_for_use.md",
    include_html = True,
)

md_template(
    name = "import_places_post",
    src = "static/posts/import_places.md",
)

md_template(
    name = "1_4_0_release_post",
    src = "static/posts/Scoria_1.4.0.md",
)

md_template(
    name = "android_release_post",
    src = "static/posts/android_release.md",
)

md_template(
    name = "first_release_post",
    src = "static/posts/first_release.md",
)

html_template(
    name = "contact",
    src = "static/contact.html",
)

md_template(
    name = "terms",
    src = "static/terms.md",
)

md_template(
    name = "unavailable",
    src = "static/unavailable.md",
)

md_template(
    name = "android_release_notes",
    src = "static/android_release_notes.md",
)

md_template(
    name = "android_notes",
    src = "static/android_notes.md",
    include_html = True,
)

# provides a convenient shorthand target name for the template files
files(
    names_srcs = {
        "feed": "static/feed.xml",
        "template_top": "static/template_top.html",
        "template_bottom": "static/template_bottom.html",
    },
)
