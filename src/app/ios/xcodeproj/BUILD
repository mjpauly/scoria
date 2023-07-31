load(
    "@rules_xcodeproj//xcodeproj:defs.bzl",
    "top_level_target",
    "xcode_schemes",
    "xcodeproj",
)

_TOP_LEVEL_TARGETS = [
    top_level_target(
        "//src/app/ios/top:Scoria",
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
            targets = ["//src/app/ios/top:Scoria"],
        ),
        launch_action = xcode_schemes.launch_action(
            "//src/app/ios/top:Scoria",
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
    project_name = "Scoria",
    schemes = _SCHEMES,
    top_level_targets = _TOP_LEVEL_TARGETS,
    visibility = ["//visibility:public"],
)

xcodeproj(
    name = "xcodeproj-focused",
    focused_targets = [
        "//src/app/ios/top:Scoria",
    ],
    project_name = "Scoria-Focused",
    schemes = _SCHEMES,
    top_level_targets = _TOP_LEVEL_TARGETS,
)
