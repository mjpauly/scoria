# Needs to match dynamic symbols in the Stem shared library
-keep class info.scoria.Stem { *; }

# These classes are looked up by name in stem_jni.cpp
-keep class info.scoria.OSLocationData { *; }
-keep class info.scoria.ServerConfig { *; }

# Make sure the JS interface poke() is not optimized out
-keep class info.scoria.WebAppInterface { void poke(); }


# warning suppression

# problems with kotlin coroutine agent and other coroutine referenced classes
-dontwarn kotlinx.coroutines.**
# another one-off warning
-dontwarn kotlin.comparisons.ComparisonsKt__ComparisonsKt

# various notes (lower severity than warnings)
-keep class kotlin.collections.SlidingWindowKt$windowedIterator$1 { int label; }
-keepclassmembers class android.os.Build$VERSION { int SDK_INT; }
