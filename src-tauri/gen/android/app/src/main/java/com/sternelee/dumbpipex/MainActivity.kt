package com.sternelee.dumbpipex

import android.Manifest
import android.content.pm.PackageManager
import android.os.Build
import android.os.Bundle
import androidx.core.app.ActivityCompat
import androidx.core.content.ContextCompat

class MainActivity : TauriActivity() {
  override fun onCreate(savedInstanceState: Bundle?) {
    super.onCreate(savedInstanceState)
    requestNearbyWifiPermission()
  }

  private fun requestNearbyWifiPermission() {
    if (Build.VERSION.SDK_INT < Build.VERSION_CODES.TIRAMISU) return

    val permission = Manifest.permission.NEARBY_WIFI_DEVICES
    if (ContextCompat.checkSelfPermission(this, permission) == PackageManager.PERMISSION_GRANTED) {
      return
    }

    ActivityCompat.requestPermissions(this, arrayOf(permission), REQUEST_NEARBY_WIFI_DEVICES)
  }

  private companion object {
    private const val REQUEST_NEARBY_WIFI_DEVICES = 4101
  }
}
