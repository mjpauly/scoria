load("@build_bazel_rules_apple//apple:apple.bzl", "local_provisioning_profile")
load("@build_bazel_rules_apple//apple:ios.bzl", "ios_application")
load("@build_bazel_rules_apple//apple:versioning.bzl", "apple_bundle_version")
load("@build_bazel_rules_swift//swift:swift.bzl", "swift_library")
load(
    "@rules_xcodeproj//xcodeproj:defs.bzl",
    "xcode_provisioning_profile",
)

ios_application(
    name = "Epsilon",
    app_icons = glob(["Resources/Assets.xcassets/AppIcon.appiconset/**"]),
    bundle_id = select({
        ":distribution_profile": "com.aperturebeam.epsilon",
        ":development_profile": "com.aperturebeam.epsilon",
        "//conditions:default": "com.matthewpauly.epsilon", # ad-hoc
    }),
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
        ":development_profile": "Epsilon_Internal.mobileprovision",
        "//conditions:default": ":ad_hoc_profile",
    }),
    resources = [
        "//src/app/stem/front:dist",
    ],
    version = ":app_version",
    visibility = ["//visibility:public"],
    deps = [":App.library"],
)

# Bump the build_version when submitting a new build to App Store Connect
apple_bundle_version(
    name = "app_version",
    build_version = "1.0.2",
    short_version_string = "1.0",
)

xcode_provisioning_profile(
    name = "ad_hoc_profile",
    managed_by_xcode = True,
    provisioning_profile = ":local_profile",
    tags = ["manual"],
)

local_provisioning_profile(
    name = "local_profile",
    profile_name = "iOS Team Provisioning Profile: com.matthewpauly.epsilon",
    tags = ["manual"],
    team_id = "QVSG4RRSDG",
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
#   `--define profile=development`
# omitting the flag causes the build to use an ad-hoc profile
config_setting(
    name = "distribution_profile",
    values = {
        "define": "profile=distribution",
    },
    visibility = ["//visibility:public"],
)

config_setting(
    name = "development_profile",
    values = {
        "define": "profile=development",
    },
    visibility = ["//visibility:public"],
)
