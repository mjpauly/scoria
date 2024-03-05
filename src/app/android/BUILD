load("@rules_kotlin//kotlin:android.bzl", "kt_android_library")
load("@rules_kotlin//kotlin:core.bzl", "kt_compiler_plugin")

android_binary(
    name = "app",
    manifest = "AndroidManifest.xml",
    manifest_values = {"applicationId": "info.scoria"},
    deps = [":lib"],
)

kt_android_library(
    name = "lib",
    srcs = glob(["kt/*.kt"]),
    custom_package = "info.scoria",
    manifest = "LibraryManifest.xml",
    resource_files = glob(["res/**/*"], exclude = ["**/.DS_Store"]),
    deps = [
        "//src/app/android/jni:stem_java",
        "@maven//:androidx_activity_activity",
        "@maven//:androidx_annotation_annotation",
        "@maven//:androidx_appcompat_appcompat",
    ],
)

# install the debug app (app.apk) with adb and run it
sh_binary(
    name = "debug",
    srcs = ["debug.sh"],
    data = [":app"],
)
