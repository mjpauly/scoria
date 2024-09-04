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
import android.view.View
import android.view.ViewGroup.MarginLayoutParams
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

        // permissions may change even while in the background
        updateLocationConfig()

        setContentView(R.layout.activity_main)

        val serverConf = Stem.handleEnterForeground()
        val uscope = serverConf.scope.toULong()
        val url = "http://127.0.0.1:${serverConf.port}/${uscope}/"
        // Log.i(TAG, "connecting to ${url}")

        val wv = findViewById<WebView>(R.id.webview)
        setInsets(wv)
        wv.webViewClient = CustomWebViewClient(this)
        wv.getSettings().javaScriptEnabled = true
        wv.addJavascriptInterface(
            WebAppInterface({ handlePoke() }, this), "Android"
        )
        wv.loadUrl(url)
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
            // Don't want the window insets to pass down to descendant views
            WindowInsetsCompat.CONSUMED
        }
    }

    override fun onStop() {
        super.onStop()
        Log.i(TAG, "onStop")
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
            shareWithName(
                this,
                srcPath,
                getDbExportName(),
            )
        }
    }

    // Check if we should import a Sqlite log, and do so
    private fun checkImport() {
        if (Stem.shouldImportSqliteLog()) {
            initiateImport(this, IMPORT_SQLITE_CODE)
        } else if (Stem.shouldImportPlacesGeojson()) {
            initiateImport(this, IMPORT_PLACES_GEOJSON_CODE)
        }
    }

    // Check if a track should be exported, and do it
    private fun checkExportTrack() {
        if (Stem.shouldExportTrack()) {
            exportTrack(this)
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
