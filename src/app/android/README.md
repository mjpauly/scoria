# Scoria Android App

Based on the [Bazel Kotlin example](https://github.com/bazelbuild/examples/tree/d18ce42623f136616e9512d713a47c33f9ad6a58/android/jetpack-compose)

## Start Emulator

```
$ANDROID_HOME/emulator/emulator -list-avds
$ANDROID_HOME/emulator/emulator -avd Medium_Phone_API_34
```

View logs

```
$ANDROID_HOME/platform-tools/adb logcat
```

## Building

Build and run the app in the emulator:

```
bazel build :app --config=android
bazel run :run_debug --config=android -- -s
```

The run_debug target can be fed `-s` to start the app after installing, and
`-d` to delete the previous install and data. They can be combined into `-ds`.

It may be necessary to make the sh binary runnable with `chmod +x run_debug.sh`.

There are issues with bazel's `mobile-install` command and how it puts
resources and native libraries in different locations on the device, so install
is done the "traditional" way with `adb`.

Build the jni lib on its own:

```
bazel build shim:jni_lib --platforms=//:android_aarch64
```

## Release

### Apk

```
# Build, zipalign, and sign
bazel build app --config=android_release
zipalign -v -p 4 bazel-bin/src/app/android/app_unsigned.apk release/app_aligned.apk
apksigner sign --ks ~/keystores/distribution_keystore.jks --out release/scoria.apk release/app_aligned.apk

# Save the unsigned app, proguard mapping, etc for future debug
bazel build package_release --config=android_release
cp bazel-bin/src/app/andriod/android_relase_x.y.zip .

# Check alignment, signature, and that debug is off (0x0 is off)
zipalign -v -c 4 release/scoria.apk
apksigner verify -v release/scoria.apk
aapt dump xmltree app_unsigned.apk AndroidManifest.xml | grep debug
```

View the cert with `keytool -printcert -jarfile scoria_1_3_g.apk`. Or for in
the keystore directy with
`keytool -v -list -keystore ~/keystores/distribution_keystore.jks`.

### Bundle

`app.aab` contains the proguard mapping in the bundle metadata, which is not
put into the apks generated for clients. Perhaps play console helps with
de-obfuscation of stack traces here, but for now we just upload the version
without the mapping, `app_deployable.aab`.

```
bazel build app_deployable --config=android_release

# Save the app_deployable for uploading to play console
cp bazel-bin/src/app/android/app_deployable.aab .

# Save the unsigned app, proguard mapping, etc for future debug
bazel build package_release --config=android_release
cp bazel-bin/src/app/andriod/android_relase_x.y.zip .

# sign the deployable bundle ("upload" is the key alias)
jarsigner -keystore ~/keystores/upload_keystore.jks app_deployable.aab upload
# verify
jarsigner -verify app_deployable.aab
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

## Inspect Manifest Values

Compiled manifest values can be inspected with:

```
aapt dump xmltree app_unsigned.apk AndroidManifest.xml
```

## Open URL Scheme

```
$ANDROID_HOME/platform-tools/adb shell 'am start -a android.intent.action.VIEW -d "scoria://place?lng=-122&lat=37&name=hi"'
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

`ld.lld: error: undefined symbol`: check presence of `#[no_mangle]

### Can't Mock Emulator Locations

Open Google Maps and allow it to use location.
