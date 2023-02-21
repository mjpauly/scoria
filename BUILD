# Global aliases for the workspace

# Builds the Xcode project
alias(
    name = "xcodeproj",
    actual = "//src/app/ios/xcodeproj",
)

# Builds/runs the ios app in the simulator
alias(
    name = "iosapp",
    actual = "//src/app/ios/top:App",
)

# Generates a `rust_project.json` which makes Rust Analyzer work in our
# non-Cargo workspace
alias(
    name = "rustanalyzer",
    actual = "@rules_rust//tools/rust_analyzer:gen_rust_project",
)


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
