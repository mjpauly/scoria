package info.scoria

import java.util.Base64
import android.util.Log
import android.os.Bundle
import android.webkit.WebView 
import androidx.activity.enableEdgeToEdge
import androidx.appcompat.app.AppCompatActivity
import android.Manifest
import androidx.activity.result.contract.ActivityResultContracts

class MainActivity : AppCompatActivity() {

    // private var customLocationManager: CustomLocationManager? = null

    private var locationTrack: LocationTrack? = null

    private var foregroundGranted = false
    private var backgroundGranted = false
    private val locationPermissionRequest = registerForActivityResult(
        ActivityResultContracts.RequestMultiplePermissions()
    ) { permissions ->
        when {
            permissions.getOrDefault(Manifest.permission.ACCESS_FINE_LOCATION, false) -> {
                // Precise location access granted.
                foregroundGranted = true
            }
            permissions.getOrDefault(Manifest.permission.ACCESS_COARSE_LOCATION, false) -> {
                // Only approximate location access granted.
                foregroundGranted = true
            }
            permissions.getOrDefault(Manifest.permission.ACCESS_BACKGROUND_LOCATION, false) -> {
                // Only approximate location access granted.
                backgroundGranted = true
            }
        }
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        Log.i("info.scoria.lifecycle", "onCreate")
        enableEdgeToEdge()
        Stem.handleStartup(
            getFilesDir().getAbsolutePath(),
            getCacheDir().getAbsolutePath()
        )

        if (locationTrack == null) {
            System.out.println("location track didn't exist")
            locationTrack = LocationTrack(this)
        }
        updateLocationConfig()
    }

    public override fun onDestroy() {
        super.onDestroy()
        Log.i("info.scoria.lifecycle", "onDestroy")
        // TODO: if logging location, stop the listener if we've just down
        // locationTrack?.stopListener() // prevents rotation?
        Stem.handleShutdown()
    }

    public override fun onStart() {
        super.onStart()
        Log.i("info.scoria.lifecycle", "onStart")

        setContentView(R.layout.activity_main)

        val serverConf = Stem.handleEnterForeground()
        val uscope = serverConf.scope.toULong()
        System.out.println("${serverConf.port}")
        System.out.println("${uscope}")
        val url = "http://127.0.0.1:${serverConf.port}/${uscope}/"

        val wv = findViewById<WebView>(R.id.webview)
        wv.webViewClient = CustomWebViewClient(this)
        wv.getSettings().javaScriptEnabled = true
        wv.addJavascriptInterface(
            WebAppInterface({ handlePoke() }), "Android"
        )
        wv.loadUrl(url)
    }

    public override fun onStop() {
        super.onStop()
        Log.i("info.scoria.lifecycle", "onStop")
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


    override fun onRestart() {
        super.onRestart()
        Log.i("info.scoria.lifecycle", "onRestart")
    }
    override fun onPause() {
        super.onPause()
        Log.i("info.scoria.lifecycle", "onPause")
    }
    override fun onResume() {
        super.onResume()
        Log.i("info.scoria.lifecycle", "onResume")
    }
    override fun onSaveInstanceState(outState: Bundle) {
        super.onSaveInstanceState(outState);
        Log.i("info.scoria.lifecycle", "onSaveInstanceState")
    }
    override fun onDetachedFromWindow() {
        super.onDetachedFromWindow()
        Log.i("info.scoria.lifecycle", "onDetachedFromWindow")
    }

    fun handlePoke() {
        // WebView callbacks normally run on another thread, and this causes
        // a SELinux denial when calling into the Stem shared library, so we
        // make sure we do things on the main UI thread
        runOnUiThread {
            System.out.println("got poke!")
            checkForegroundPermissions()
            updateLocationConfig()
            // locationTrack?.startListener()
            // System.out.println("${locationTrack?.loc?.getLongitude()}, ${locationTrack?.loc?.getLatitude()}")
        }
    }

    // If we should request foreground location access, do that
    fun checkForegroundPermissions() {
        if (Stem.shouldRequestWhenInUseAuthorization())  {
            locationPermissionRequest.launch(arrayOf(
                Manifest.permission.ACCESS_FINE_LOCATION,
                Manifest.permission.ACCESS_COARSE_LOCATION)
            )
        }
    }

    // Enable/disable location
    fun updateLocationConfig() {
        System.out.println("updating config");
        if (Stem.getLocationEnabled()) {
            locationTrack?.startListener()
        } else {
            locationTrack?.stopListener()
        }
    }
}

/* Log location:
val loc = OSLocationData()
loc.timestamp = 44
loc.latitude = 1.0;
loc.longitude = 1.0;
loc.horizontal_accuracy = 1.0;
loc.msl_altitude = 1.0;
loc.ellipsoid_altitude = 1.0;
loc.vertical_accuracy = 1.0;
loc.story_available = false;
loc.story = -1;
loc.speed = 1.0;
loc.speed_accuracy = 1.0;
loc.course = 1.0;
loc.course_accuracy = 1.0;
loc.source_info_available = true;
loc.is_simulated_by_software = false;
loc.is_produced_by_accessory = false;
Stem.logLocation(loc)

    @Preview
    @Composable
    fun HelloWorld(name: String) = Column(
        verticalArrangement = Arrangement.Center,
        horizontalAlignment = Alignment.CenterHorizontally,
        modifier = Modifier
            .fillMaxSize()
            .padding(20.dp)) {
        Text(
            text = "Hello $name",
            textAlign = TextAlign.Center
        )
    }
*/
