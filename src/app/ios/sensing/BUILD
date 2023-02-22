load("@build_bazel_rules_apple//apple:ios.bzl", "ios_framework")
load("@build_bazel_rules_swift//swift:swift.bzl", "swift_library")

swift_library(
    name = "sensing",
    srcs = glob(["**/*.swift"]),
    data = glob(  # TODO: move Logo.png over to top level Resources
        ["Resources/**"],
        exclude = ["**/.DS_Store"],
    ),
    module_name = "Sensing",
    tags = ["manual"],
    visibility = ["//visibility:public"],
    deps = [
        "//src/app/stem:StemLib",
    ],
    alwayslink = True,
)

ios_framework(
    name = "sensing_framework",
    bundle_id = "com.matthewpauly.epsilon.sensing",
    families = ["iphone"],
    infoplists = ["Info.plist"],
    minimum_os_version = "16.0",
    visibility = ["//visibility:public"],
    deps = [":sensing"],
)
