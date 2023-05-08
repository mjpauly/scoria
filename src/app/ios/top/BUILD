load("@build_bazel_rules_apple//apple:ios.bzl", "ios_application")
load("@build_bazel_rules_apple//apple:versioning.bzl", "apple_bundle_version")
load("@build_bazel_rules_swift//swift:swift.bzl", "swift_library")

ios_application(
    name = "Epsilon",
    app_icons = glob(["Resources/Assets.xcassets/AppIcon.appiconset/**"]),
    bundle_id = "com.aperturebeam.epsilon",
    bundle_name = "Epsilon",
    families = ["iphone"],
    frameworks = [
        "//src/app/ios/sensing:sensing_framework",
    ],
    infoplists = [":Info.plist"],
    launch_storyboard = "Resources/Launch.storyboard",
    minimum_os_version = "16.0",
    provisioning_profile = select({
        ":distribution_profile": "Epsilon_iOS_Distribution.mobileprovision",
        "//conditions:default": "Epsilon_Internal.mobileprovision",
    }),
    version = ":app_version",
    visibility = ["//visibility:public"],
    deps = [":App.library"],
)

# Bump the build_version when submitting a new build to App Store Connect
apple_bundle_version(
    name = "app_version",
    build_version = "1.0.3",
    short_version_string = "1.0",
)

swift_library(
    name = "App.library",
    srcs = glob(["*.swift"]),
    module_name = "Epsilon",
    tags = ["manual"],
    visibility = ["//src/app/ios/tests:__subpackages__"],
    deps = [
        "//src/app/ios/sensing",
    ],
)

# command line flags for setting the provisioning profile to build the bundle:
#   `--define profile=distribution`
# omitting the flag causes the build to use the development profile
config_setting(
    name = "distribution_profile",
    values = {
        "define": "profile=distribution",
    },
    visibility = ["//visibility:public"],
)
