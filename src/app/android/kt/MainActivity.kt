package info.scoria

import android.Manifest.permission
import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.content.ServiceConnection
import android.content.pm.PackageManager.PERMISSION_GRANTED
import android.os.Binder
import android.os.Build
import android.os.Bundle
import android.os.IBinder
import android.util.Log
import android.view.HapticFeedbackConstants
import android.view.View
import android.view.ViewGroup.MarginLayoutParams
import android.view.WindowManager
import android.webkit.WebView 
import androidx.activity.enableEdgeToEdge
import androidx.activity.result.contract.ActivityResultContracts
import androidx.appcompat.app.AppCompatActivity
import androidx.core.app.ActivityCompat
import androidx.core.content.ContextCompat
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat
import androidx.core.view.updateLayoutParams
import java.lang.Math.max
import java.nio.file.Path
import java.util.Base64
import kotlin.io.path.Path

class MainActivity : AppCompatActivity(), UpdateConfigCallback {

    private val TAG = "MainActivity"

    private var mService: LocationService? = null
    private var mServiceIntent: Intent? = null
    private var serverUrl: String? = null
    private var started = false
    private var safeTopCssPx: Float = 0f
    private var safeLeftCssPx: Float = 0f
    private var safeRightCssPx: Float = 0f
    private var safeBottomCssPx: Float = 0f

    private val locationPermissionRequest = registerForActivityResult(
        ActivityResultContracts.RequestMultiplePermissions()
    ) { permissions ->
        when {
            permissions.getOrDefault(permission.ACCESS_FINE_LOCATION, false) -> {
                Log.i(TAG, "foreground fine granted")
            }
            permissions.getOrDefault(permission.ACCESS_COARSE_LOCATION, false) -> {
                Log.i(TAG, "foreground coarse granted")
            }
            permissions.getOrDefault(permission.ACCESS_BACKGROUND_LOCATION, false) -> {
                Log.i(TAG, "background granted")
            }
        }
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        Log.i(TAG, "onCreate")
        enableEdgeToEdge()
        // Draw into the display cutout in landscape instead of letterboxing
        // with a black bar; the page keeps clear of it via the injected
        // safe-inset variables.
        window.attributes.layoutInDisplayCutoutMode =
            WindowManager.LayoutParams.LAYOUT_IN_DISPLAY_CUTOUT_MODE_SHORT_EDGES
        Stem.handleStartup(
            getFilesDir().getAbsolutePath(),
            getCacheDir().getAbsolutePath(),
            getVersionName(this)
        )
        Stem.setVersionCode(getVersionCode(this))
        updateLocationConfig()
        cleanupAllSharedFiles(this)
    }

    override fun onDestroy() {
        super.onDestroy()
        Log.i(TAG, "onDestroy")
        Stem.handleShutdown()
    }

    override fun onStart() {
        super.onStart()
        Log.i(TAG, "onStart")
        started = true

        // permissions may change even while in the background
        updateLocationConfig()

        setContentView(R.layout.activity_main)

        val serverConf = Stem.handleEnterForeground()
        val uscope = serverConf.scope.toULong()
        serverUrl = "http://127.0.0.1:${serverConf.port}/${uscope}/"
        // Log.i(TAG, "connecting to ${serverUrl}")
        initWebView(serverUrl!!)
    }

    private fun initWebView(url: String) {
        val wv = findViewById<WebView>(R.id.webview)
        setInsets(wv)
        wv.webViewClient = CustomWebViewClient(
            this,
            { handleRendererGone() },
            { injectSafeInsets(it) },
        )
        wv.getSettings().javaScriptEnabled = true
        // Suppress the system long-press buzz the WebView performs on its
        // own gesture detection; it fires on any press-and-hold, even over
        // blank areas. Our deliberate haptics go through the decor view in
        // handleHaptic, so they are unaffected.
        wv.isHapticFeedbackEnabled = false
        wv.addJavascriptInterface(
            WebAppInterface({ handlePoke() }, { handleHaptic(it) }, this),
            "Android"
        )
        wv.loadUrl(url)
    }

    // The dead webview was already destroyed; re-inflate the layout for a
    // fresh one and reload. While stopped, skip: the UI server may be down,
    // and onStart rebuilds the webview anyway.
    private fun handleRendererGone() {
        if (!started) return
        setContentView(R.layout.activity_main)
        serverUrl?.let { initWebView(it) }
    }

    /* Setup the view with the desired inserts */
    fun setInsets(view: View) {
        ViewCompat.setOnApplyWindowInsetsListener(view) { v, windowInsets ->
            val sysBarInsets =
                windowInsets.getInsets(WindowInsetsCompat.Type.systemBars())
            val keyboardInsets =
                windowInsets.getInsets(WindowInsetsCompat.Type.ime())
            // Apply the insets as a margin to the view.
            var mlp = v.getLayoutParams() as MarginLayoutParams
            mlp.leftMargin = sysBarInsets.left
            mlp.bottomMargin = max(sysBarInsets.bottom, keyboardInsets.bottom)
            mlp.rightMargin = sysBarInsets.right
            v.setLayoutParams(mlp)
            // The webview extends under the status bar / cutout at the top
            // and under the cutout at the sides in landscape. WebViews older
            // than 140 report env(safe-area-inset-*) as 0, so pass the insets
            // to the page as CSS variables as well. The margins above already
            // keep the webview clear of the system bars, so only the part of
            // a cutout reaching past them needs offsetting in CSS.
            val cutoutInsets =
                windowInsets.getInsets(WindowInsetsCompat.Type.displayCutout())
            val density = resources.displayMetrics.density
            safeTopCssPx = max(sysBarInsets.top, cutoutInsets.top) / density
            safeLeftCssPx =
                max(0, cutoutInsets.left - mlp.leftMargin) / density
            safeRightCssPx =
                max(0, cutoutInsets.right - mlp.rightMargin) / density
            safeBottomCssPx =
                max(0, cutoutInsets.bottom - mlp.bottomMargin) / density
            injectSafeInsets(v as WebView)
            // Don't want the window insets to pass down to descendant views
            WindowInsetsCompat.CONSUMED
        }
    }

    // Expose the insets to the page as --safe-* variables. Also re-run on
    // page load: the properties don't survive a (re)load, and the insets
    // listener may fire before the page exists.
    fun injectSafeInsets(wv: WebView) {
        wv.evaluateJavascript(
            "var s = document.documentElement.style;" +
                "s.setProperty('--safe-top', '${safeTopCssPx}px');" +
                "s.setProperty('--safe-left', '${safeLeftCssPx}px');" +
                "s.setProperty('--safe-right', '${safeRightCssPx}px');" +
                "s.setProperty('--safe-bottom', '${safeBottomCssPx}px');",
            null
        )
    }

    override fun onStop() {
        super.onStop()
        Log.i(TAG, "onStop")
        started = false
        Stem.handleEnterBackground()

        // free webview resources by loading a blank black page
        val unencodedHtml: String =
            "<html><body style=\"background-color: black;\"></body></html>"
        val encodedHtml = 
            Base64
                .getEncoder()
                .withoutPadding()
                .encodeToString(unencodedHtml.toByteArray())
        val wv = findViewById<WebView>(R.id.webview)
        wv.loadData(encodedHtml, "text/html", "base64");
    }

    // url scheme intents delivered after onStart
    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        Log.i(TAG, "onNewIntent")
        intent.data?.let {
            Stem.urlScheme(it.toString());
        }
    }

    // Haptic feedback requested by the webapp. performHapticFeedback
    // respects the system haptics setting, unlike a raw Vibrator.
    private fun handleHaptic(kind: String) {
        val constant = when (kind) {
            "tick" -> HapticFeedbackConstants.CLOCK_TICK
            "hold" -> HapticFeedbackConstants.LONG_PRESS
            // "prepare" only matters on iOS
            else -> return
        }
        // JavascriptInterface calls arrive on a background thread. The decor
        // view, not the webview: view haptics are disabled on the webview to
        // mute the system long-press buzz.
        runOnUiThread {
            window.decorView.performHapticFeedback(constant)
        }
    }

    private fun handlePoke() {
        // WebView callbacks normally run on another thread, and this causes
        // a SELinux denial when calling into the Stem shared library, so we
        // make sure we do things on the main UI thread
        runOnUiThread {
            // Log.i(TAG, "got poke!")
            checkForegroundPermissions()
            checkLocationSourceSetting()
            updateLocationConfig()
            checkExportSqliteLog()
            checkImport()
            checkExportTrack()
            checkExportImage()
            checkRequestNotifications()
        }
    }

    // Request foreground location access if needed. Does not block.
    private fun checkForegroundPermissions() {
        if (Stem.shouldRequestWhenInUseAuthorization())  {
            locationPermissionRequest.launch(arrayOf(
                permission.ACCESS_FINE_LOCATION,
                permission.ACCESS_COARSE_LOCATION,
            ))
        }
    }

    private fun checkLocationSourceSetting() {
        if (Stem.shouldGoToLocationSettings()) {
            startActivity(
                Intent(
                    android.provider.Settings.ACTION_LOCATION_SOURCE_SETTINGS
                )
            )
        }
    }

    // Copy data.db to sharable path in the cache dir with a new name, and
    // share it from there, then delete it
    private fun checkExportSqliteLog() {
        if (Stem.shouldExportSqliteLog()) {
            val db_fname = "data.db"
            val base = Path(getFilesDir().getAbsolutePath())
            val srcPath = base.resolve(db_fname)
            val exportName = getDbExportName()
            // Rust checkpoints the WAL into the database file before setting
            // the flag, truncating it to zero bytes on success, so a large
            // one means the checkpoint could not run and its frames only
            // exist there: share it alongside, named so SQLite finds it next
            // to the database. The threshold leaves room for the few
            // locations logged since the checkpoint (~4 KB frame each).
            val walPath = base.resolve(db_fname + "-wal")
            val extraFiles = if (walPath.toFile().length() > 100_000) {
                listOf(walPath to (exportName + "-wal"))
            } else {
                emptyList()
            }
            shareWithName(
                this,
                srcPath,
                exportName,
                extraFiles,
            )
        }
    }

    // Check if we should import a Sqlite log, and do so
    private fun checkImport() {
        if (Stem.shouldImportSqliteLog()) {
            initiateImport(this, IMPORT_SQLITE_CODE)
        } else if (Stem.shouldImportPlacesGeojson()) {
            initiateImport(this, IMPORT_PLACES_GEOJSON_CODE)
        } else if (Stem.shouldImportMountedDB()) {
            initiateImport(this, IMPORT_MOUNTED_DB_CODE)
        }
    }

    // Check if a track should be exported, and do it
    private fun checkExportTrack() {
        if (Stem.shouldExportTrack()) {
            exportTrack(this)
        }
    }

    private fun checkExportImage() {
        if (Stem.shouldExportImage()) {
            exportImage(this)
        }
    }

    // Check if notification permissions should be requested
    private fun checkRequestNotifications() {
        if (Stem.shouldNotifyOnStop()) {
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
                if (ContextCompat.checkSelfPermission(
                    this,
                    permission.POST_NOTIFICATIONS
                ) != PERMISSION_GRANTED) {
                    ActivityCompat.requestPermissions(
                        this,
                        arrayOf(permission.POST_NOTIFICATIONS),
                        1
                    )
                }
            }
        }
    }

    public override fun onActivityResult(
        requestCode: Int,
        resultCode: Int,
        returnIntent: Intent?
    ) {
        if (requestCode == SHARE_CODE) {
            cleanupSharedFile()
        } else if (resultCode == RESULT_OK) {
            if (requestCode == IMPORT_SQLITE_CODE) {
                returnIntent?.data?.also { returnUri ->
                    completeSqliteImport(this, returnUri)
                }
            } else if (requestCode == IMPORT_PLACES_GEOJSON_CODE) {
                returnIntent?.data?.also { returnUri ->
                    completePlacesGeojsonImport(this, returnUri)
                }
            } else if (requestCode == IMPORT_MOUNTED_DB_CODE) {
                returnIntent?.data?.also { returnUri ->
                    completeMountedDBImport(this, returnUri)
                }
            }
        }
    }

    // Enable/disable location
    private fun updateLocationConfig() {
        if (Stem.getLocationEnabled() && foregroundGranted()) {
            Intent(this, LocationService::class.java).also { intent ->
                bindService(intent, connection, Context.BIND_AUTO_CREATE)
                startService(intent)
                mServiceIntent = intent
            }
        }
        if (!Stem.getLocationEnabled()) {
            mServiceIntent?.let {
                unbindService(connection)
                stopService(it)
            }
            mServiceIntent = null
        }
    }

    // Callbacks for service binding used by `bindService`
    private val connection = object : ServiceConnection {
        override fun onServiceConnected(componentName: ComponentName, service: IBinder) {
            val binder = service as LocationService.LocalBinder
            mService = binder.getService()
            mService?.registerCallback(this@MainActivity) // Register the callback
        }

        override fun onServiceDisconnected(componentName: ComponentName) {
            mService = null
        }
    }

    public override fun updateConfigCallback() {
        runOnUiThread {
            // Log.i(TAG, "updating location config from service")
            updateLocationConfig()
        }
    }

    private fun foregroundGranted(): Boolean {
        return (checkSelfPermission(permission.ACCESS_COARSE_LOCATION)
            == PERMISSION_GRANTED)
    }

    override fun onRestart() {
        super.onRestart()
        Log.i(TAG, "onRestart")
    }
    override fun onPause() {
        super.onPause()
        Log.i(TAG, "onPause")
    }
    override fun onResume() {
        super.onResume()
        Log.i(TAG, "onResume")
    }
    override fun onSaveInstanceState(outState: Bundle) {
        super.onSaveInstanceState(outState);
        Log.i(TAG, "onSaveInstanceState")
    }
    override fun onDetachedFromWindow() {
        super.onDetachedFromWindow()
        Log.i(TAG, "onDetachedFromWindow")
    }
}

// callback interface for the service
interface UpdateConfigCallback {
    fun updateConfigCallback()
}

fun getVersionName(context: Context): String {
    return context.getPackageManager()
        .getPackageInfo(context.getPackageName(), 0)
        .versionName
}

fun getVersionCode(context: Context): Long {
    return context.getPackageManager()
        .getPackageInfo(context.getPackageName(), 0)
        .getLongVersionCode()
}
