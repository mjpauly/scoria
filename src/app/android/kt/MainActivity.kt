package info.scoria

import android.os.Bundle
import androidx.activity.compose.setContent
import androidx.appcompat.app.AppCompatActivity
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp

import android.view.ViewGroup 
import android.webkit.WebView 
import android.webkit.WebViewClient 
import androidx.activity.ComponentActivity 
import androidx.compose.material.* 
import androidx.compose.ui.viewinterop.AndroidView
import androidx.activity.enableEdgeToEdge

class MainActivity : AppCompatActivity() {

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        Stem.handleStartup(
            getFilesDir().getAbsolutePath(),
            getCacheDir().getAbsolutePath()
        )
        val serverConf = Stem.handleEnterForeground()
        System.out.println("${serverConf.port}");
        System.out.println("${serverConf.scope}");
        Stem.handleEnterBackground()
        Stem.handleShutdown()


        // setContent { HelloWorld("World") }
        setContent { HelloWorld(Stem.stringFromJNI()) }
        // setContent { WebViewScreen() }
    }

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

    @Composable
    fun WebViewScreen() {
        AndroidView(
            factory = { context ->
                WebView(context).apply {
                    webViewClient = WebViewClient()
                    settings.javaScriptEnabled = true
                }
            },
            update = { webView ->
                webView.loadUrl("https://www.wikipedia.org/")
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
*/
