package com.example.android.bazel

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

// getFilesDir().getAbsolutePath(),
// getCacheDir().getAbsolutePath(),
class MainActivity : AppCompatActivity() {

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()

        // setContent { HelloWorld("World") }
        // setContent { HelloWorld(JniShim.stringFromJNI()) }
        setContent { WebViewScreen() }
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
