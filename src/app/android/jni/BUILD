package(default_visibility = ["//src/app/android:__pkg__"])

android_library(
    name = "stem_java",
    srcs = glob(["*.java"]),
    deps = [":stem_jni"],
)

# Build with --platforms=//:android_aarch64 to inspect .so shared library file
# View symbols with `nm x.so` and `nm -D x.so` (dynamic symbols)
cc_library(
    name = "stem_jni",
    hdrs = [
        "jni_bind_release.h",
        "logging.h"
    ],
    srcs = [
        "stem_jni.cpp",
        "logging.cpp"
    ],
    deps = ["//src/app/stem:stem_cc"],

    defines = select({
        "//:distribution_profile": [],
        "//conditions:default": ["DEBUG_LOGGING"],
    }),

    # Don't export symbols from static lib dependencies as dynamic symbols, and
    # strip all non-dynamic symbols.
    linkopts = select({
        "//:distribution_profile": ["-Wl,--exclude-libs=ALL", "-Wl,-s"],
        "//conditions:default": [],
    }),
)

# Download jni_bind_release.h header v0.9.9
# Ideally we'd depend on @jni-bind//:jni_bind instead, but the @local_jdk deps
# for the target aren't working, and we can just use the flattened release
# header anyways.
sh_binary(
    name = "download_jni_bind",
    srcs = ["download_jni_bind.sh"],
)
