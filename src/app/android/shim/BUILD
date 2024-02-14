package(default_visibility = ["//src/app/android:__pkg__"])

android_library(
    name = "jni_shim",
    srcs = ["JniShim.java"],
    deps = [
        ":jni_lib",
    ],
)

cc_library(
    name = "jni_lib",
    srcs = ["native-lib.cpp"],
)
