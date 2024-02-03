load("@build_bazel_rules_apple//apple:ios.bzl", "ios_extension")
load("@build_bazel_rules_swift//swift:swift.bzl", "swift_library", "swift_binary")
load("@build_bazel_rules_apple//apple:apple.bzl", "local_provisioning_profile")
load("@rules_xcodeproj//xcodeproj:defs.bzl", "xcode_provisioning_profile")


ios_extension(
    name = "widget",
    bundle_id = "com.aperturebeam.epsilon.widget",
    bundle_name = "ScoriaWidget",
    families = ["iphone"],
    infoplists = [":Info.plist"],
    minimum_os_version = "16.0",
    provisioning_profile = select({
        "//:distribution_profile": "Scoria_Widget_Distribution.mobileprovision",
        "//conditions:default": "Scoria_Widget_Internal.mobileprovision",
    }),
    resources = glob(["Assets.xcassets/AppIcon.appiconset/**"]),
    version = "//src/app/ios/top:app_version",
    visibility = ["//src/app/ios:__subpackages__"],
    deps = [
        ":widget_lib",
    ],
)

swift_library(
    name = "widget_lib",
    srcs = glob(["*.swift"]),
    data = glob(
        ["Assets.xcassets/**"],
        exclude = ["**/.DS_Store"],
    ),
    module_name = "ScoriaWidget",
)
