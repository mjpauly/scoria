load("@build_bazel_rules_swift//swift:swift.bzl", "swift_library")

swift_library(
    name = "sensing",
    srcs = glob(["**/*.swift"]),
    module_name = "Sensing",
    visibility = ["//visibility:public"],
    deps = [
        "//src/app/stem:StemLib",
    ],
)
