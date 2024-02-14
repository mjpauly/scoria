# Scoria Android App

Based on the [Bazel Kotlin example](https://github.com/bazelbuild/examples/tree/d18ce42623f136616e9512d713a47c33f9ad6a58/android/jetpack-compose)

## Building

Build and upload the app to the simulator:

```
bazel build :app --fat_apk_cpu=arm64-v8a --android_crosstool_top=@androidndk//:toolchain  --noincompatible_enable_cc_toolchain_resolution
bazel mobile-install :app --fat_apk_cpu=arm64-v8a --start_app --android_crosstool_top=@androidndk//:toolchain --noincompatible_enable_cc_toolchain_resolution
```

Build the jni lib on its own:

```
bazel build //app/src/main:jni_lib --incompatible_enable_cc_toolchain_resolution --platforms=//app:arm64-v8a
```
