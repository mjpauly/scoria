package info.scoria

import android.Manifest.permission
import android.content.Intent
import android.content.pm.PackageManager.PERMISSION_GRANTED
import android.os.Bundle
import android.util.Log
import android.webkit.WebView 
import android.os.IBinder
import android.content.Context
import androidx.activity.enableEdgeToEdge
import androidx.activity.result.contract.ActivityResultContracts
import android.content.ComponentName
import androidx.appcompat.app.AppCompatActivity
import java.util.Base64
import android.os.Binder
import android.content.ServiceConnection

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
            getCacheDir().getAbsolutePath()
        )
        updateLocationConfig()
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
        wv.webViewClient = CustomWebViewClient(this)
        wv.getSettings().javaScriptEnabled = true
        wv.addJavascriptInterface(
            WebAppInterface({ handlePoke() }), "Android"
        )
        wv.loadUrl(url)
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

    private fun handlePoke() {
        // WebView callbacks normally run on another thread, and this causes
        // a SELinux denial when calling into the Stem shared library, so we
        // make sure we do things on the main UI thread
        runOnUiThread {
            // Log.i(TAG, "got poke!")
            checkForegroundPermissions()
            updateLocationConfig()
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
