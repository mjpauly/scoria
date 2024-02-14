package com.example.android.bazel;

public class JniShim {
    static {
        System.loadLibrary("app"); // same as name of andriod_binary target
    }
    public static native String stringFromJNI();
}
