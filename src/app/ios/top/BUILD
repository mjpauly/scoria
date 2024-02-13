load("@build_bazel_rules_apple//apple:ios.bzl", "ios_application")
load("@build_bazel_rules_apple//apple:versioning.bzl", "apple_bundle_version")
load("@build_bazel_rules_swift//swift:swift.bzl", "swift_library")

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

# Bump the build_version when submitting a new build to App Store Connect
# 
# Convention: short version string is the first part of the build version. When
# working towards a new release, bump the minor version number for both,
# resetting the patch number on the build version. E.g.:
# 
# Builds leading up to release "7.5" would be "7.5.0", "7.5.1", "7.5.2", ...
# Once "7.5" is released publicly, the next build versions should become "7.6.0"
#
# NOTE: The public 7.5 release does NOT necessarily correspond to 7.5.0, but to
# the LAST build number matching "7.5.x"
apple_bundle_version(
    name = "app_version",
    build_version = "1.3.7",
    short_version_string = "1.3",
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
