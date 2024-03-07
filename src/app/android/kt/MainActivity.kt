package info.scoria

import android.Manifest.permission
import android.content.Intent
import android.content.pm.PackageManager.PERMISSION_GRANTED
import android.os.Bundle
import android.util.Log
import android.webkit.WebView 
import androidx.activity.enableEdgeToEdge
import androidx.activity.result.contract.ActivityResultContracts
import androidx.appcompat.app.AppCompatActivity
import java.util.Base64

class MainActivity : AppCompatActivity() {

    private val TAG = "MainActivity"

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
            mServiceIntent = Intent(this, LocationService::class.java)
            // restarted a running service will only cause onStartCommand to be
            // called again, so no need to guard with a check on the run state
            startService(mServiceIntent)
        }
        if (!Stem.getLocationEnabled()) {
            mServiceIntent?.let { stopService(it) }
        }
    }

    private fun foregroundGranted(): Boolean {
        return (checkSelfPermission(permission.ACCESS_COARSE_LOCATION)
            == PERMISSION_GRANTED)
    }

    // private fun backgroundGranted(): Boolean {
        // return (checkSelfPermission(permission.ACCESS_BACKGROUND_LOCATION)
            // == PERMISSION_GRANTED)
    // }

    // Request background location access if needed. Does not block.
    // private fun checkBackgroundPermissions() {
        // if (foregroundGranted() && !backgroundGranted())  {
            // locationPermissionRequest.launch(arrayOf(
                // permission.ACCESS_BACKGROUND_LOCATION,
            // ))
        // }
    // }

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
