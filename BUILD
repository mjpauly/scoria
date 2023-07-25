# Global aliases for the workspace

# Builds the Xcode project
alias(
    name = "xcodeproj",
    actual = "//src/app/ios/xcodeproj",
)

# Builds/runs the ios app in the simulator
alias(
    name = "iosapp",
    actual = "//src/app/ios/top:Epsilon",
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

# Config for optimized builds. Sometimes useful during development, so use
# //:distribution_profile for settings that need to be set only for public
# release.
config_setting(
    name = "optimized_build",
    values = {"compilation_mode": "opt"}
)

# Command line flag for setting the provisioning profile to build the bundle:
#   `--define profile=distribution`
# Omitting the flag causes the build to use the development profile.
#
# Also used to turn off debug logging and use the production maptiler key.
config_setting(
    name = "distribution_profile",
    values = {
        "define": "profile=distribution",
    },
    visibility = ["//visibility:public"],
)
