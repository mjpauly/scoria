# Scoria Android App

Based on the [Bazel Kotlin example](https://github.com/bazelbuild/examples/tree/d18ce42623f136616e9512d713a47c33f9ad6a58/android/jetpack-compose)

## Building

Build and run the app in the emulator:

```
bazel build :app --config=android
bazel run :debug --config=android -- -s
```

The debug target can be fed `-s` to start the app after installing, and `-d` to
delete the previous install and data. They can be combined into `-ds`.

It may be necessary to make the sh binary runnable with `chmod +x debug.sh`.

There are issues with bazel's `mobile-install` command and how it puts
resources and native libraries in different locations on the device, so install
is done the "traditional" way with `adb`.

Build the jni lib on its own:

```
bazel build shim:jni_lib --platforms=//:android_aarch64
```

## JNI

https://www.baeldung.com/jni

Given the Java function prototypes, the native function prototypes can be
generated with:

```
javac -h . *.java
```

Copy them from the generated .h file into `stem_jni.cpp`.

The compiled `.class` files are also outputted. Use `javap -s Stem.class` to
inspect the JNI interface descriptors.

## Emulator Locations

Update the location in the emulator with:

```
bazel run test:emu
```

or manually with:

```
$ANDROID_HOME/platform-tools/adb emu geo fix -122.0 35.0
```

## Common Issues

### Thread Access to Stem

The Stem shared library can only be accessed from the app's main thread. If we
try to access it from another thread, we get an SELinux denial and a big red
stack trace. Notably it will warn `avc:  denied` before the stacktrace. To
debug, it's useful to see what code is running on what thread, by printing the
thread names:

```
Log.d(TAG, "on thread: ${java.lang.Thread.currentThread().getName()}")
```

### jni_bind Errors

Make sure to build with the `--config=android{_release}` flag!
