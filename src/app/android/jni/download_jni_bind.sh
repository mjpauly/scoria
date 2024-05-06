#!/bin/bash
set -euo pipefail
IFS=$'\n\t'

cd $BUILD_WORKSPACE_DIRECTORY/src/app/android/jni

curl https://raw.githubusercontent.com/google/jni-bind/22ffc67cc0d57f20934c3094dc71e46bb978a75d/jni_bind_release.h -o jni_bind_release.h
