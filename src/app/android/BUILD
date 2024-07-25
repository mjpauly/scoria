load("@rules_kotlin//kotlin:android.bzl", "kt_android_library")
load("@rules_kotlin//kotlin:core.bzl", "kt_compiler_plugin")
load("@rules_android_app_bundles//android:android_application.bzl",
    "android_application", "deployable_android_application"
)
load("//:src/app/version.bzl", "ANDROID_VERSION_NAME", "ANDROID_VERSION_CODE")
load("@bazel_skylib//rules:select_file.bzl", "select_file")

proguard_specs = select({
    "//:distribution_profile": [
        "proguard/proguard-rules.pro",
        "proguard/proguard-android-optimize.txt-8.2.2",
    ],
    "//conditions:default": [],
})

genrule(
    name = "aligned_apk",
    srcs = [":unaligned_apk"],
    outs = ["Scoria_" + ANDROID_VERSION_NAME + ".apk"],
    cmd = """
zipalign -v -p 4 $(location :unaligned_apk) $@

# verify
zipalign -v -c 4 $@

# check debuggable is set to 0x0 for release (0xffffffff for when debuggable)
aapt dump xmltree $@ AndroidManifest.xml | grep debug | grep -v 0xffffffff \
    || { echo "ERROR: apk is debuggable"; false; }
""",
)

select_file(
    name = "unaligned_apk",
    srcs = ":app",
    subpath = "app_unsigned.apk",
)

# Generates a app binary (.apk).
android_binary(
    name = "app",
    manifest = "AndroidManifest.xml",
    manifest_values = {
        "applicationId": "info.scoria",
        "versionName": ANDROID_VERSION_NAME,
        "versionCode": ANDROID_VERSION_CODE,
    },
    deps = [":lib"],
    proguard_specs = proguard_specs,
    proguard_generate_mapping = True, # create mapping file of obfuscated names
)

kt_android_library(
    name = "lib",
    srcs = glob(["kt/*.kt"]),
    custom_package = "info.scoria",
    manifest = select({
        # Not sure of an easier way to toggle the "debuggable" attribute.
        # Supposedly `-c opt` used to flip it false, but this is no longer true.
        "//:distribution_profile": "LibraryManifestRelease.xml",
        "//conditions:default": "LibraryManifestDebug.xml",
    }),
    resource_files = glob(["res/**/*"], exclude = ["**/.DS_Store"]),
    deps = [
        "//src/app/android/jni:stem_java",
        "@maven//:androidx_activity_activity",
        "@maven//:androidx_annotation_annotation",
        "@maven//:androidx_appcompat_appcompat",
    ],
)

# Generate an app bundle (.aab) for use with Google Play. Creates these targets:
# :app_aab               contains proguard mapping in non-release metadata
# :app_deployable        release bundle (only created if proguard_ is True)
# :app_module_aab        proto formatted resources
# :app_module_zip        proto formatted and zipped
# :app_univeral_apks     based on :app_bundle_aab -> contains mapping?
#       note the typo! (no s in "univeral")
# Note that .aab files are zip files like .apk files, and can be inspected
# similarly
android_application(
    target = "app", # used to generate target names, so cannot include the ":"
    proguard_generate_mapping = True, # turn off for non-release bundle tests
    app_bundle_config_file = "bundle/config.pb.json",
)

# installs (but does not run) the app bundle (.aab)
deployable_android_application(
    name = "install_app_bundle",
    aab_target = ":app_aab", # cannot be app_deployable for non-release builds
)

# install the debug app (app.apk) with adb and run it
sh_binary(
    name = "run_debug",
    srcs = ["run_debug.sh"],
    data = [":app"],
)

# package up the app, proguard mapping, jar classes, etc for future debug
genrule(
    name = "package_release",
    srcs = [":app"],
    outs = ["android_release_" + ANDROID_VERSION_NAME + ".zip"],
    cmd = select({
        "//:distribution_profile": "zip $@ $(locations :app)",
        "//conditions:default": "exit 1", # fail if not doing a release build
    }),
)
