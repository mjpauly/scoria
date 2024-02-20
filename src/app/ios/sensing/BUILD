load("@build_bazel_rules_swift//swift:swift.bzl", "swift_library")

swift_library(
    name = "sensing",
    srcs = glob(["**/*.swift"]),
    module_name = "Sensing",
    visibility = ["//visibility:public"],
    deps = [":StemLib"],
)

# The objective C library we can import into Swift
objc_library(
    name = "StemLib",
    hdrs = ["StemLib.h"],
    module_name = "StemLib",
    deps = ["//src/app/stem:stem_cc"],
)

