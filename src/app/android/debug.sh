#!/bin/bash
set -euo pipefail
IFS=$'\n\t'


d_flag=''
s_flag=''

print_usage() {
  printf "Usage: debug.sh [-d] [-s]
        -d: delete previous install and data
        -s: start app after install\n"
}

while getopts 'ds' flag; do
  case "${flag}" in
    d) d_flag='true' ;;
    s) s_flag='true' ;;
    *) print_usage
       exit 1 ;;
  esac
done


if [ $d_flag = 'true' ] ; then
    echo "Deleting previous app and data"
    $ANDROID_HOME/platform-tools/adb uninstall info.scoria
fi

echo "Installing app"
$ANDROID_HOME/platform-tools/adb install src/app/android/app.apk

if [ $s_flag = 'true' ] ; then
    echo "Starting app"
    $ANDROID_HOME/platform-tools/adb shell monkey -p info.scoria 1
fi
