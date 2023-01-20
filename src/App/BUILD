load("@build_bazel_rules_apple//apple:apple.bzl", "local_provisioning_profile")
load("@build_bazel_rules_apple//apple:ios.bzl", "ios_application")
load("@build_bazel_rules_swift//swift:swift.bzl", "swift_library")
load(
    "@com_github_buildbuddy_io_rules_xcodeproj//xcodeproj:defs.bzl",
    "xcode_provisioning_profile",
)

# Change these to have code signing work for you
BUNDLE_ID = "com.matthewpauly.epsilon"

MANAGED_PROFILE = True

PROFILE_NAME = "iOS Team Provisioning Profile: {}".format(BUNDLE_ID)

TEAMID = "QVSG4RRSDG"

ios_application(
    name = "App",
    app_icons = glob(["Resources/Assets.xcassets/AppIcon.appiconset/**"]),
    bundle_id = BUNDLE_ID,
    bundle_name = "Epsilon",
    families = ["iphone"],
    frameworks = [
        "//src/UI:UIFramework",
    ],
    infoplists = [":Info.plist"],
    launch_storyboard = "Resources/Launch.storyboard",
    minimum_os_version = "16.0",
    provisioning_profile = select({
        ":device_build": ":xcode_profile",
        "//conditions:default": None,
    }),
    visibility = ["//visibility:public"],
    deps = [":App.library"],
)

xcode_provisioning_profile(
    name = "xcode_profile",
    managed_by_xcode = MANAGED_PROFILE,
    provisioning_profile = ":local_profile",
    tags = ["manual"],
)

local_provisioning_profile(
    name = "local_profile",
    profile_name = PROFILE_NAME,
    tags = ["manual"],
    team_id = TEAMID,
)

swift_library(
    name = "App.library",
    srcs = [":gen_App.swift"],
    module_name = "App",
    tags = ["manual"],
    visibility = ["//src/AppTests:__subpackages__"],
    deps = [
        "//src/UI",
    ],
)

genrule(
    name = "gen_App.swift",
    outs = ["App.swift"],
    cmd = """\
cat > $@ <<EOF
import SwiftUI
import UI

let internalValue = 42

@main
struct iOSApp: App {
    var body: some Scene {
        WindowGroup {
            ContentView()
        }
    }
}
EOF
""",
    tags = ["manual"],
)

config_setting(
    name = "device_build",
    values = {
        "cpu": "ios_arm64",
    },
    visibility = ["//visibility:public"],
)
