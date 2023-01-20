load(
    "@com_github_buildbuddy_io_rules_xcodeproj//xcodeproj:defs.bzl",
    "top_level_target",
    "xcode_schemes",
    "xcodeproj",
)

_TOP_LEVEL_TARGETS = [
    top_level_target(
        "//src/App",
        target_environments = [
            "device",
            "simulator",
        ],
    ),
    "//src/AppTests",
    "//src/OtherTests",
    "//src/RustLib:tests",
]

_SCHEMES = [
    xcode_schemes.scheme(
        name = "App",
        build_action = xcode_schemes.build_action(
            targets = ["//src/App"],
        ),
        launch_action = xcode_schemes.launch_action(
            "//src/App",
        ),
        test_action = xcode_schemes.test_action(
            [
                "//src/AppTests",
                "//src/OtherTests",
            ],
            diagnostics = xcode_schemes.diagnostics(
                sanitizers = xcode_schemes.sanitizers(
                    thread = True,
                ),
            ),
        ),
    ),
]

xcodeproj(
    name = "xcodeproj",
    project_name = "Epsilon",
    schemes = _SCHEMES,
    top_level_targets = _TOP_LEVEL_TARGETS,
)

xcodeproj(
    name = "xcodeproj-focused",
    focused_targets = [
        "//src/App",
    ],
    project_name = "Epsilon-Focused",
    schemes = _SCHEMES,
    top_level_targets = _TOP_LEVEL_TARGETS,
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
