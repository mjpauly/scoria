/* WebViewClient that determines whether to open a url in the same web view or
 * to open it in a browser.
 *
 * Example: https://www.digitalocean.com/community/tutorials/android-webview-example-tutorial
 */

package info.scoria

import android.app.Activity;
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.os.Build
import android.util.Log
import android.webkit.WebResourceRequest
import android.webkit.WebView 
import android.webkit.WebViewClient 
import androidx.annotation.RequiresApi


class CustomWebViewClient(parentActivity: Activity) : WebViewClient() {

    private var activity: Activity

    init {
        activity = parentActivity
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

    private fun handleUri(uri: Uri): Boolean {
        Log.i("scoria.info", "Uri =" + uri)
        val host: String = uri.getHost()!!
        val scheme: String = uri.getScheme()!!
        System.out.println("host: ${host}, scheme: ${scheme}")
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
