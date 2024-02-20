# Scoria Android App

Based on the [Bazel Kotlin example](https://github.com/bazelbuild/examples/tree/d18ce42623f136616e9512d713a47c33f9ad6a58/android/jetpack-compose)

## Building

Build and upload the app to the simulator:

```
bazel build :app --config=android
bazel mobile-install :app --config=android --start_app
```

Build the jni lib on its own:

```
bazel build shim:jni_lib --platforms=//:android_aarch64
```
