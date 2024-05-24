load("@build_bazel_rules_apple//apple:ios.bzl", "ios_application")
load("@build_bazel_rules_apple//apple:versioning.bzl", "apple_bundle_version")
load("@build_bazel_rules_swift//swift:swift.bzl", "swift_library")
load("//:src/app/version.bzl", "IOS_VERSION_NAME", "IOS_BUILD_VERSION")

ios_application(
    name = "Scoria",
    app_icons = glob(
        ["Resources/Assets.xcassets/AppIcon.appiconset/**"],
        exclude = ["**/.DS_Store"],
    ),
    bundle_id = "com.aperturebeam.epsilon",
    bundle_name = "Scoria",
    # optional, since provisioning profile supports the App groups capability,
    # but redundantly including it helps surface inconsistencies
    entitlements = "Scoria.entitlements",
    extensions = ["//src/app/ios/widget"],
    families = ["iphone", "ipad"],
    infoplists = [":Info.plist"],
    launch_storyboard = "Resources/Launch.storyboard",
    minimum_os_version = "16.0",
    provisioning_profile = select({
        "//:distribution_profile": "Scoria_Distribution.mobileprovision",
        "//conditions:default": "Scoria_Internal.mobileprovision",
    }),
    version = ":app_version",
    visibility = ["//visibility:public"],
    deps = [":App.library"],
)

apple_bundle_version(
    name = "app_version",
    build_version = IOS_BUILD_VERSION,
    short_version_string = IOS_VERSION_NAME,
    visibility = ["//src/app/ios:__subpackages__"],
)

swift_library(
    name = "App.library",
    srcs = glob(["*.swift"]),
    module_name = "Scoria",
    visibility = ["//src/app/ios/tests:__subpackages__"],
    deps = [
        "//src/app/ios/sensing",
    ],
    defines = select({
        # disallow debugging the webview in app distribution
        "//:distribution_profile": [],
        "//conditions:default": ["DEBUG_WEBVIEW"],
    }),
)
