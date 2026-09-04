/* WebViewClient that determines whether to open a url in the same web view or
 * to open it in a browser.
 *
 * Example: https://www.digitalocean.com/community/tutorials/android-webview-example-tutorial
 */

package info.scoria

import android.app.Activity;
import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.os.Build
import android.util.Log
import android.view.ViewGroup
import android.webkit.JavascriptInterface
import android.webkit.RenderProcessGoneDetail
import android.webkit.WebResourceRequest
import android.webkit.WebView
import android.webkit.WebViewClient
import androidx.annotation.RequiresApi


class CustomWebViewClient(
    private val activity: Activity,
    private val onRendererGone: () -> Unit,
    private val onPageLoaded: (WebView) -> Unit,
) : WebViewClient() {

    private val TAG = "WebViewClient"

    override fun onPageFinished(view: WebView?, url: String?) {
        view?.let(onPageLoaded)
    }

    @SuppressWarnings("deprecation")
    public override fun shouldOverrideUrlLoading(
        view: WebView,
        url: String
    ): Boolean {
        val uri: Uri = Uri.parse(url)
        return handleUri(uri)
    }

    @RequiresApi(Build.VERSION_CODES.N)
    public override fun shouldOverrideUrlLoading(
        view: WebView,
        request: WebResourceRequest
    ): Boolean {
        val uri: Uri = request.getUrl()
        return handleUri(uri)
    }

    // The webview's renderer process died (killed under memory pressure, a
    // crash, etc.). Returning true keeps our process alive, but this WebView
    // instance is dead and must be destroyed; the callback builds a fresh
    // one. Backend state lives in our process, so the page comes back to
    // the same view. See doc/decimation/memory-limits.md, "Webview
    // resilience and leaks".
    override fun onRenderProcessGone(
        view: WebView,
        detail: RenderProcessGoneDetail
    ): Boolean {
        Log.w(TAG, "WebView renderer gone, didCrash=${detail.didCrash()}")
        (view.parent as? ViewGroup)?.removeView(view)
        view.destroy()
        onRendererGone()
        return true
    }

    private fun handleUri(uri: Uri): Boolean {
        Log.i(TAG, "Uri =" + uri)
        val host: String = uri.getHost()!!
        // val scheme: String = uri.getScheme()!!
        // Log.i(TAG, "host: ${host}, scheme: ${scheme}")
        if (host.startsWith("127.0.0.1")) {
            // load this url in the webView itself
            return false
        } else {
            // open web page in a browser
            val intent: Intent = Intent(Intent.ACTION_VIEW, uri)
            activity.startActivity(intent);
            return true
        }
    }
}

class WebAppInterface(
    val callback: () -> Unit,
    val hapticCallback: (String) -> Unit,
    private val context: Context,
) {

    @JavascriptInterface
    fun poke() {
        callback()
    }

    @JavascriptInterface
    fun haptic(kind: String) {
        hapticCallback(kind)
    }
    
    // Android WebView doesn't implement clipboard, so we use an escape hatch.
    @JavascriptInterface
    fun copyToClipboard(text: String) {
        val clipboard = context.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
        val clip = ClipData.newPlainText("demo", text)
        clipboard.setPrimaryClip(clip)
    }
}

