load("@build_bazel_rules_apple//apple:ios.bzl", "ios_unit_test")
load("@build_bazel_rules_swift//swift:swift.bzl", "swift_library")

ios_unit_test(
    name = "OtherTests",
    bundle_id = "com.matthewpauly.epsilon.OtherTests",
    minimum_os_version = "16.0",
    visibility = ["//visibility:public"],
    deps = [":OtherTests.library"],
)

swift_library(
    name = "OtherTests.library",
    testonly = True,
    srcs = glob(["**/*.swift"]),
    module_name = "OtherTests",
    deps = ["@com_acme//:SuperSecret"],
    tags = ["manual"],
)
