#include <jni.h>
#include <string>

extern "C" {
#include "../../stem/stem.h"
}

extern "C"
JNIEXPORT jstring

JNICALL
Java_com_example_android_bazel_JniShim_stringFromJNI(
        JNIEnv *env,
        jobject /* this */) {
    std::string hello;
    if (get_bool()) {
        hello = "C++";
    } else {
        hello = "D++";
    }
    return env->NewStringUTF(hello.c_str());
}
