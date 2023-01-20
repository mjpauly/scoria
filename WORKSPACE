load("@bazel_tools//tools/build_defs/repo:http.bzl", "http_archive")


# rules_xcodeproj

http_archive(
    name = "com_github_buildbuddy_io_rules_xcodeproj",
    sha256 = "b4e71c7740bb8cfa4bc0b91c0f18ac512debcc111ebe471280e24f579a3b0782",
    url = "https://github.com/buildbuddy-io/rules_xcodeproj/releases/download/0.10.2/release.tar.gz",
)

load(
    "@com_github_buildbuddy_io_rules_xcodeproj//xcodeproj:repositories.bzl",
    "xcodeproj_rules_dependencies",
)

xcodeproj_rules_dependencies()


# apple / swift support

http_archive(
    name = "build_bazel_rules_apple",
    sha256 = "43737f28a578d8d8d7ab7df2fb80225a6b23b9af9655fcdc66ae38eb2abcf2ed",
    url = "https://github.com/bazelbuild/rules_apple/releases/download/2.0.0/rules_apple.2.0.0.tar.gz",
)

load(
    "@build_bazel_rules_apple//apple:repositories.bzl",
    "apple_rules_dependencies",
)

apple_rules_dependencies()

load(
    "@build_bazel_rules_swift//swift:repositories.bzl",
    "swift_rules_dependencies",
)

swift_rules_dependencies()

load(
    "@build_bazel_rules_swift//swift:extras.bzl",
    "swift_rules_extra_dependencies",
)

swift_rules_extra_dependencies()

load(
    "@build_bazel_apple_support//lib:repositories.bzl",
    "apple_support_dependencies",
)

apple_support_dependencies()

load(
    "@build_bazel_rules_apple//apple:apple.bzl",
    "provisioning_profile_repository",
)

provisioning_profile_repository(
    name = "local_provisioning_profiles",
)


# rules_rust

load("@bazel_tools//tools/build_defs/repo:http.bzl", "http_archive")
http_archive(
    name = "rules_rust",
    sha256 = "aaaa4b9591a5dad8d8907ae2dbe6e0eb49e6314946ce4c7149241648e56a1277",
    urls = ["https://github.com/bazelbuild/rules_rust/releases/download/0.16.1/rules_rust-v0.16.1.tar.gz"],
)

load(
    "@rules_rust//rust:repositories.bzl",
    "rules_rust_dependencies",
    "rust_register_toolchains",
)

rules_rust_dependencies()

rust_register_toolchains(
    extra_target_triples = [
        "aarch64-apple-ios-sim",
        "aarch64-apple-ios",
        "x86_64-apple-ios",
    ],
)


# third party repo

local_repository(
    name = "com_acme",
    path = "fake_third_party/com_acme",
)
