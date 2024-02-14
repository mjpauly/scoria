#include <jni.h>
#include <string>

extern "C"
JNIEXPORT jstring

JNICALL
Java_com_example_android_bazel_JniShim_stringFromJNI(
        JNIEnv *env,
        jobject /* this */) {
    std::string hello = "C++";
    // std:: // uncomment for compile error
    return env->NewStringUTF(hello.c_str());
}
