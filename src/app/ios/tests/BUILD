load("@build_bazel_rules_apple//apple:ios.bzl", "ios_unit_test")
load("@build_bazel_rules_swift//swift:swift.bzl", "swift_library")

ios_unit_test(
    name = "AppTests",
    bundle_id = "com.matthewpauly.epsilon.AppTests",
    minimum_os_version = "16.0",
    test_host = "//src/app/ios/top:Scoria",
    visibility = ["//visibility:public"],
    deps = [":AppTests.library"],
)

swift_library(
    name = "AppTests.library",
    testonly = True,
    srcs = glob(["**/*.swift"]),
    module_name = "AppTests",
    tags = ["manual"],
    deps = [
        "//src/app/ios/top:App.library",
    ],
)
