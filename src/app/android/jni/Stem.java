/* Java wrapper around our native C interface.
 */
package info.scoria;

public class Stem {
    static {
        System.loadLibrary("app"); // same as name of android_binary target
    }
    public static native void handleStartup(
        String filesDir,
        String cacheDir,
        String versionName
    );
    public static native void handleShutdown();
    public static native ServerConfig handleEnterForeground();
    public static native void handleEnterBackground();

    public static native void logError(String error);

    public static native void logLocation(OSLocationData data);

    public static native boolean getLocationEnabled();
    public static native float getDistanceFilter();
    public static native boolean getSignificantChanges();
    public static native double getLocationAccuracyMode();

    public static native boolean shouldRequestWhenInUseAuthorization();
    public static native boolean shouldGoToLocationSettings();
    public static native boolean shouldExportSqliteLog();
    public static native boolean shouldImportSqliteLog();
    public static native void importFromSqliteLog(String importPath);
    public static native boolean shouldExportTrack();
}
