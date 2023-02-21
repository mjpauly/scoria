load(
    "@com_github_buildbuddy_io_rules_xcodeproj//xcodeproj:defs.bzl",
    "top_level_target",
    "xcode_schemes",
    "xcodeproj",
)

_TOP_LEVEL_TARGETS = [
    top_level_target(
        "//src/app/ios/top:App",
        target_environments = [
            "device",
            "simulator",
        ],
    ),
    "//src/app/ios/tests:AppTests",
    "//src/app/stem:int_tests",
]

_SCHEMES = [
    xcode_schemes.scheme(
        name = "App",
        build_action = xcode_schemes.build_action(
            targets = ["//src/app/ios/top:App"],
        ),
        launch_action = xcode_schemes.launch_action(
            "//src/app/ios/top:App",
        ),
        test_action = xcode_schemes.test_action(
            [
                "//src/app/ios/tests:AppTests",
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
    visibility = ["//visibility:public"],
)

xcodeproj(
    name = "xcodeproj-focused",
    focused_targets = [
        "//src/app/ios/top:App",
    ],
    project_name = "Epsilon-Focused",
    schemes = _SCHEMES,
    top_level_targets = _TOP_LEVEL_TARGETS,
)
