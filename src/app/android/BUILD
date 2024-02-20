load("@rules_kotlin//kotlin:android.bzl", "kt_android_library")
load("@rules_kotlin//kotlin:core.bzl", "kt_compiler_plugin")

android_binary(
    name = "app",
    manifest = "AndroidManifest.xml",
    manifest_values = {"applicationId": "com.example.android.bazel"},
    deps = [":lib"],
)

kt_android_library(
    name = "lib",
    srcs = ["kt/MainActivity.kt"],
    custom_package = "com.example.android.bazel",
    manifest = "LibraryManifest.xml",
    plugins = [":jetpack_compose_compiler_plugin"],
    resource_files = glob(["res/**/*"], exclude = ["**/.DS_Store"]),
    deps = [
        "//src/app/android/shim:jni_shim",
        "@maven//:androidx_activity_activity_compose",
        "@maven//:androidx_appcompat_appcompat",
        "@maven//:androidx_compose_foundation_foundation",
        "@maven//:androidx_compose_foundation_foundation_layout",
        "@maven//:androidx_compose_runtime_runtime",
        "@maven//:androidx_compose_ui_ui",
        "@maven//:androidx_compose_ui_ui_tooling",
    ],
)

kt_compiler_plugin(
    name = "jetpack_compose_compiler_plugin",
    id = "androidx.compose.compiler",
    target_embedded_compiler = True,
    deps = ["@maven//:androidx_compose_compiler_compiler"],
)
