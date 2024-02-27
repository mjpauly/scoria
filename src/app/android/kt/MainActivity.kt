package info.scoria

import android.os.Bundle
import android.view.ViewGroup 
import android.webkit.WebView 
import android.webkit.WebViewClient 
import androidx.activity.ComponentActivity 
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.appcompat.app.AppCompatActivity
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material.* 
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView
import info.scoria.CustomWebViewClient

class MainActivity : AppCompatActivity() {

    private var webview: WebView? = null

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        Stem.handleStartup(
            getFilesDir().getAbsolutePath(),
            getCacheDir().getAbsolutePath()
        )
        System.out.println("Startup")
    }

    public override fun onDestroy() {
        super.onDestroy()
        System.out.println("Shutdown")
        Stem.handleShutdown()
    }

    public override fun onStart() {
        super.onStart()
        System.out.println("Foregrounding")
        val serverConf = Stem.handleEnterForeground()
        val uscope = serverConf.scope.toULong()
        System.out.println("${serverConf.port}")
        System.out.println("${uscope}")
        val url = "http://127.0.0.1:${serverConf.port}/${uscope}/"
        setContent { WebViewScreen(url) }
    }

    public override fun onStop() {
        super.onStop()
        System.out.println("Backgrounding")
        Stem.handleEnterBackground()

        setContent {}

        // destroy the webview to avoid memory leak
        webview?.clearHistory();
        webview?.clearCache(true);
        webview?.loadUrl("about:blank")
        webview?.onPause();
        webview?.removeAllViews();
        webview?.pauseTimers();
        // Some lifecycle stuff isn't being handled correctly here, since
        // we get this warning:
        // WebView.destroy() called while WebView is still attached to window
        webview?.destroy();
        webview = null
    }

    @Composable
    fun WebViewScreen(url: String) {
        val activity = this
        AndroidView(
            factory = { context ->
                val wv = WebView(context).apply {
                    webViewClient = CustomWebViewClient(activity)
                    settings.javaScriptEnabled = true
                    layoutParams = ViewGroup.LayoutParams(
                        ViewGroup.LayoutParams.MATCH_PARENT,
                        ViewGroup.LayoutParams.MATCH_PARENT
                    )
                }
                webview = wv // track our webview, so we can free it later
                wv
            },
            update = { webView ->
                webView.loadUrl(url)
            }
        )
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
