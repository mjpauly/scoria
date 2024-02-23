/* C++ implementations of the JNI calls.
 *
 * # JNI Bind
 *
 * https://github.com/google/jni-bind
 *
 * JNI Bind provides syntax sugar for common JNI operations. Constructing a
 * ServerConfig class with JNI Bind by calling the constructor:
 *
 * ```
 * return LocalObject<kServerConfig>{1, 2l}.Release();
 * ```
 *
 * Without JNI Bind:
 *
 * ```
 * jclass serverConfigClass = env->FindClass("info/scoria/ServerConfig");
 *
 * // Calling constructor:
 * jmethodID initServerConfig =
 *      env->GetMethodID(serverConfigClass, "<init>", "(IJ)V");
 * jobject newServerConfig =
 *      env->NewObject(serverConfigClass, initServerConfig, 1, 2l);
 *
 * // Or allocate then initialize fields without constructor:
 * jobject newServerConfig = env->AllocObject(serverConfigClass);
 * jfieldID portField = env->GetFieldID(serverConfigClass, "port", "I");
 * jfieldID scopeField = env->GetFieldID(serverConfigClass, "scope", "J");
 * env->SetIntField(newServerConfig, portField, 1);
 * env->SetLongField(newServerConfig, scopeField, 2l);
 *
 * return newServerConfig;
 * ```
 *
 *
 * ## Strings
 *
 * To conver a Java String into a null-terminated C character array, we do:
 *
 * ```
 * std::string myStr{LocalString{filesDir}.Pin().ToString()};
 * myStr.c_str()
 * ```
 *
 * While `LocalString{filesDir}.Pin().ToString()` gives a std::string_view,
 * there are no methods to get a null-terminated C character array, so we use
 * it to construct an std::string, then call c_str() on that.
 * 
 * Without JNI Bind:
 *
 * ```
 * std::string convert_java_string(JNIEnv *env, jstring s) {
 *     jsize len = env->GetStringUTFLength(s);
 *     const char *cstr = env->GetStringUTFChars(s, NULL);
 *     std::string converted = std::string(cstr, len);
 *     env->ReleaseStringUTFChars(s, cstr);
 *     return converted;
 * }
 * ```
 */
#include <jni.h>
#include <string>

#include "jni_bind_release.h"

#include "logging.h"

// include stem.h as a C ABI header
extern "C" {
#include "../../stem/stem.h"
}


using ::jni::Class;
using ::jni::Constructor;
using ::jni::Field;
using ::jni::LocalObject;
using ::jni::LocalString;

/* JNI class definitions */

static constexpr Class kServerConfig {
    "info/scoria/ServerConfig",
    Constructor{int{}, long{}},
    Field{"port", jint{}},
    Field{"scope", jlong{}},
};

static constexpr Class kOSLocationData {
    "info/scoria/OSLocationData",
    Field{"timestamp", jlong{}},
    Field{"latitude", jdouble{}},
    Field{"longitude", jdouble{}},
    Field{"horizontal_accuracy", jdouble{}},

    Field{"msl_altitude", jdouble{}},
    Field{"ellipsoid_altitude", jdouble{}},
    Field{"vertical_accuracy", jdouble{}},

    Field{"story_available", jboolean{}},
    Field{"story", jlong{}},

    Field{"speed", jdouble{}},
    Field{"speed_accuracy", jdouble{}},
    Field{"course", jdouble{}},
    Field{"course_accuracy", jdouble{}},

    Field{"source_info_available", jboolean{}},
    Field{"is_simulated_by_software", jboolean{}},
    Field{"is_produced_by_accessory", jboolean{}},
};


// export all JNI functions with a C ABI
extern "C" {


// JNI Bind book-keeping
JNIEXPORT jint JNICALL JNI_OnLoad(JavaVM* pjvm, void* reserved) {
    static auto jvm{std::make_unique<jni::JvmRef<jni::kDefaultJvm>>(pjvm)};
    return JNI_VERSION_1_6;
}

JNIEXPORT jstring JNICALL Java_info_scoria_Stem_stringFromJNI
  (JNIEnv *, jclass)
{
    std::string hello;
    if (get_bool()) {
        hello = "C++";
    } else {
        hello = "D++";
    }
    return LocalString{hello}.Release();
}

JNIEXPORT void JNICALL Java_info_scoria_Stem_handleStartup
  (JNIEnv *, jclass, jstring filesDir, jstring cacheDir)
{
    // start the stdout -> android logging thread
    start_logger("info.scoria.native"); // TODO: only in development

    std::string filesDirString{LocalString{filesDir}.Pin().ToString()};
    std::string cacheDirString{LocalString{cacheDir}.Pin().ToString()};
    set_app_dirs(
        filesDirString.c_str(), // documents
        filesDirString.c_str(), // library
        cacheDirString.c_str(), // tmp
        "", // bundle (unused)
        "x.y" // TODO
    );
}

JNIEXPORT void JNICALL Java_info_scoria_Stem_handleShutdown
  (JNIEnv *, jclass)
{
    handle_shutdown();
}

JNIEXPORT jobject JNICALL Java_info_scoria_Stem_handleEnterForeground
  (JNIEnv *, jclass)
{
    ServerConfig conf = handle_enter_foreground();
    LocalObject<kServerConfig> kconf {(int)conf.port, (long)conf.scope};
    return kconf.Release();
}

JNIEXPORT void JNICALL Java_info_scoria_Stem_handleEnterBackground
  (JNIEnv *, jclass)
{
    handle_enter_background();
}

JNIEXPORT void JNICALL Java_info_scoria_Stem_logError
  (JNIEnv *, jclass, jstring errorString)
{
    std::string converted{LocalString{errorString}.Pin().ToString()};
    log_error(converted.c_str());
}

JNIEXPORT void JNICALL Java_info_scoria_Stem_logLocation
  (JNIEnv *, jclass, jobject data)
{
    LocalObject<kOSLocationData> localData{data};
    OSLocationData cdata {
        localData["timestamp"].Get(),
        localData["latitude"].Get(),
        localData["longitude"].Get(),
        localData["horizontal_accuracy"].Get(),
        localData["msl_altitude"].Get(),
        localData["ellipsoid_altitude"].Get(),
        localData["vertical_accuracy"].Get(),
        (bool)localData["story_available"].Get(),
        localData["story"].Get(),
        localData["speed"].Get(),
        localData["speed_accuracy"].Get(),
        localData["course"].Get(),
        localData["course_accuracy"].Get(),
        (bool)localData["source_info_available"].Get(),
        (bool)localData["is_simulated_by_software"].Get(),
        (bool)localData["is_produced_by_accessory"].Get(),
    };
    log_location(cdata);
}

JNIEXPORT jboolean JNICALL Java_info_scoria_Stem_getLocationEnabled
  (JNIEnv *, jclass)
{
    return get_location_enabled();
}

JNIEXPORT jfloat JNICALL Java_info_scoria_Stem_getDistanceFilter
  (JNIEnv *, jclass)
{
    return get_distance_filter();

}

JNIEXPORT jboolean JNICALL Java_info_scoria_Stem_getSignificantChanges
  (JNIEnv *, jclass)
{
    return get_significant_changes();
}

JNIEXPORT jdouble JNICALL Java_info_scoria_Stem_getLocationAccuracyMode
  (JNIEnv *, jclass)
{
    LocAccuracyMode mode = get_location_accuracy_mode();
    double ret;
    switch (mode) {
        case Best: ret = 0.0; break;
        case TenMeters: ret = 10.0; break;
        case HundredMeters: ret = 100.0; break;
        case Kilometer: ret = 1000.0; break;
        case ThreeKilometers: ret = 3000.0; break;
    }
    return ret;
}

JNIEXPORT jboolean JNICALL Java_info_scoria_Stem_shouldRequestWhenInUseAuthorization
  (JNIEnv *, jclass)
{
    return should_request_when_in_use_authorization();

}

JNIEXPORT jboolean JNICALL Java_info_scoria_Stem_shouldExportSqliteLog
  (JNIEnv *, jclass)
{
    return should_export_sqlite_log();
}

JNIEXPORT jboolean JNICALL Java_info_scoria_Stem_shouldImportSqliteLog
  (JNIEnv *, jclass)
{
    return should_import_sqlite_log();
}

JNIEXPORT void JNICALL Java_info_scoria_Stem_importFromSqliteLog
  (JNIEnv *, jclass, jstring path)
{
    std::string converted{LocalString{path}.Pin().ToString()};
    import_from_sqlite_log(converted.c_str());
}

JNIEXPORT jboolean JNICALL Java_info_scoria_Stem_shouldExportTrack
  (JNIEnv *, jclass)
{
    return should_export_track();
}


} // extern "C"
