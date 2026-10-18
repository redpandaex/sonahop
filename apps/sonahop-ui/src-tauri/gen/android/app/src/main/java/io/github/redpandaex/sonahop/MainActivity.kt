package io.github.redpandaex.sonahop

import android.content.res.Configuration
import android.graphics.Color
import android.os.Bundle
import android.view.View
import android.webkit.WebView
import androidx.activity.enableEdgeToEdge
import androidx.core.graphics.Insets
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat
import androidx.core.view.doOnAttach

class MainActivity : TauriActivity() {
  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)

    val content = findViewById<View>(android.R.id.content)
    updateCanvasBackground()
    ViewCompat.setOnApplyWindowInsetsListener(content) { view, windowInsets ->
      val handledTypes = WindowInsetsCompat.Type.systemBars() or WindowInsetsCompat.Type.displayCutout()
      val insets = windowInsets.getInsets(handledTypes)
      view.setPadding(insets.left, insets.top, insets.right, insets.bottom)
      // Keep IME and other inset updates flowing to WebView after padding the safe area.
      WindowInsetsCompat.Builder(windowInsets)
        .setInsets(handledTypes, Insets.NONE)
        .build()
    }
    requestContentInsets()
  }

  override fun onWebViewCreate(webView: WebView) {
    super.onWebViewCreate(webView)
    // Wry attaches the WebView after onCreate; request insets once it can receive them.
    webView.doOnAttach { requestContentInsets() }
  }

  override fun onConfigurationChanged(newConfig: Configuration) {
    super.onConfigurationChanged(newConfig)
    enableEdgeToEdge()
    updateCanvasBackground()
    requestContentInsets()
  }

  private fun updateCanvasBackground() {
    val nightMode = resources.configuration.uiMode and Configuration.UI_MODE_NIGHT_MASK
    val canvas = if (nightMode == Configuration.UI_MODE_NIGHT_YES) "#0f141c" else "#f5f7fb"
    findViewById<View>(android.R.id.content).setBackgroundColor(Color.parseColor(canvas))
  }

  private fun requestContentInsets() {
    val content = findViewById<View>(android.R.id.content)
    content.post { ViewCompat.requestApplyInsets(content) }
  }
}
