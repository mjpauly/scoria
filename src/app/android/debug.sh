# install the app with adb
$ANDROID_HOME/platform-tools/adb install src/app/android/app.apk
# start the app
$ANDROID_HOME/platform-tools/adb shell monkey -p info.scoria 1
