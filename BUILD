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
    name = "android_aarch64",
    constraint_values = [
        "@platforms//cpu:aarch64",
        "@platforms//os:android",
    ],
)

platform(
    name = "android_armeabi",
    constraint_values = [
        "@platforms//cpu:armv7",
        "@platforms//os:android",
    ],
)

platform(
    name = "android_x86",
    constraint_values = [
        "@platforms//cpu:x86_32",
        "@platforms//os:android",
    ],
)

platform(
    name = "android_x86_64",
    constraint_values = [
        "@platforms//cpu:x86_64",
        "@platforms//os:android",
    ],
)

# Build Configuration
# ===================
#
# There are three types of build configuration:
# - the platform type (ios/android)
# - the optimization level (fastbuild/opt)
# - the distribution type (internal/release)
#
# The defaults are `ios`, `fastbuild`, and `internal`. Each build configuration
# type is propagated to targets in different ways.
# 
# Many targets, such as the WASM frontend, do not know what the architecture of
# the upstream targets is. So platform-specific configuration is propagated
# using a `string_flag` and `config_setting`. E.g. `--//:platform_config=ios`
# and `--//:platform_config=android`. While `target_os` would work for native
# targets, we use feature flags specifically for readability and so
# configurations can be decoupled from the target architecture, like when
# running the dev setup on macos.
# 
# The optimization level can be propagated to each target without needing to
# set an additional string flag, since the `optimized_build` config_setting
# depends on `-c opt` directly.
#
# The distribution profile is only propagated using the string_flag and
# `distribution_profile` config_setting.

# Config for optimized builds.
config_setting(
    name = "optimized_build",
    values = { "compilation_mode": "opt" },
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
# - disables webview debugging
# - enables android native library symbol stripping
config_setting(
    name = "distribution_profile",
    flag_values = { ":profile": "distribution", },
    visibility = ["//visibility:public"],
)

# set --//:autoreload=on to enable autoreload during development
string_flag(
    name = "autoreload",
    build_setting_default = "off",
)

config_setting(
    name = "autoreload_on",
    flag_values = { ":autoreload": "on" },
    visibility = ["//visibility:public"],
)

# Flag for iOS vs Android build, so we can change things in WASM, where it's not
# apparent what the upstream target platform is.
string_flag(
    name = "platform_config",
    build_setting_default = "ios",
)

config_setting(
    name = "android_config",
    flag_values = { ":platform_config": "android" },
    visibility = ["//visibility:public"],
)
