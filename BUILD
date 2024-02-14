# Global aliases for the workspace
load("@bazel_skylib//rules:common_settings.bzl", "string_flag")

# Builds the Xcode project
alias(
    name = "xcodeproj",
    actual = "//src/app/ios/xcodeproj",
)

# Builds/runs the ios app in the simulator
alias(
    name = "iosapp",
    actual = "//src/app/ios/top:Scoria",
)

# Generates a `rust_project.json` which makes Rust Analyzer work in our
# non-Cargo workspace
alias(
    name = "rustanalyzer",
    actual = "@rules_rust//tools/rust_analyzer:gen_rust_project",
)


# Secrets are placed in a .env file which and are not tracked in source control.
filegroup(
    name = "secrets",
    srcs = [".env"],
    visibility = ["//visibility:public"],
)

# default rustfmt settings
exports_files(["rustfmt.toml"])


# Platform definitions that are used in the 'platform_mappings' file

platform(
    name = "ios_x86_64",
    constraint_values = [
        "@platforms//cpu:x86_64",
        "@platforms//os:ios",
        "@build_bazel_apple_support//constraints:simulator",
    ],
)

platform(
    name = "ios_sim_arm64",
    constraint_values = [
        "@platforms//cpu:arm64",
        "@platforms//os:ios",
        "@build_bazel_apple_support//constraints:simulator",
    ],
)

platform(
    name = "ios_arm64",
    constraint_values = [
        "@platforms//cpu:arm64",
        "@platforms//os:ios",
        "@build_bazel_apple_support//constraints:device",
    ],
)

platform(
    name = "arm64-v8a",
    constraint_values = [
        "@platforms//cpu:arm64",
        "@platforms//os:android",
    ],
)

# Config for optimized builds.
config_setting(
    name = "optimized_build",
    values = {
        "compilation_mode": "opt"
    },
    visibility = ["//visibility:public"],
)

# Flag for the distribution profile.
string_flag(
    name = "profile",
    build_setting_default = "internal",
)

# Command line flag for enabling settings which are used when compiling the app
# for distribution. Use like so:
#   `--//:profile=distribution`
#
# Effects:
# - enables the distribution provisioning profile to build/sign the app bundle
# - enables the production maptiler key
# - disables debug logging
# - disables webview debugging
config_setting(
    name = "distribution_profile",
    flag_values = {
        ":profile": "distribution",
    },
    visibility = ["//visibility:public"],
)

# set --//:autoreload=on to enable autoreload during development
string_flag(
    name = "autoreload",
    build_setting_default = "off",
)

config_setting(
    name = "autoreload_on",
    flag_values = {
        ":autoreload": "on"
    },
    visibility = ["//visibility:public"],
)
