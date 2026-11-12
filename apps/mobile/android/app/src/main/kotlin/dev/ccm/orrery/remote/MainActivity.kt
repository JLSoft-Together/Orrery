package dev.ccm.orrery.remote

import android.app.NotificationChannel
import android.app.NotificationManager
import android.os.Build
import android.app.KeyguardManager
import android.content.Intent
import android.os.Bundle
import android.os.SystemClock
import android.provider.Settings
import android.view.WindowManager
import io.flutter.embedding.android.FlutterFragmentActivity
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.plugin.common.MethodChannel

class MainActivity : FlutterFragmentActivity() {
    override fun configureFlutterEngine(flutterEngine: FlutterEngine) {
        super.configureFlutterEngine(flutterEngine)
        MethodChannel(flutterEngine.dartExecutor.binaryMessenger, PRIVACY_CHANNEL)
            .setMethodCallHandler { call, result ->
                when (call.method) {
                    "setSecure" -> {
                        val secure = call.argument<Boolean>("secure") == true
                        if (secure) {
                            window.addFlags(WindowManager.LayoutParams.FLAG_SECURE)
                        } else {
                            window.clearFlags(WindowManager.LayoutParams.FLAG_SECURE)
                        }
                        result.success(null)
                    }
                    "isDeviceSecure" -> {
                        val keyguard = getSystemService(KeyguardManager::class.java)
                        result.success(keyguard?.isDeviceSecure)
                    }
                    "elapsedRealtime" -> result.success(SystemClock.elapsedRealtime())
                    "openSecuritySettings" -> result.success(openSecuritySettings())
                    else -> result.notImplemented()
                }
            }
    }

    private fun openSecuritySettings(): Boolean = try {
        startActivity(Intent(Settings.ACTION_SECURITY_SETTINGS).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
        true
    } catch (error: Exception) {
        false
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        createAttentionChannel()
    }

    private fun createAttentionChannel() {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.O) return
        val manager = getSystemService(NotificationManager::class.java) ?: return
        val channel = NotificationChannel(
            ATTENTION_CHANNEL,
            getString(R.string.attention_channel_name),
            NotificationManager.IMPORTANCE_HIGH,
        )
        channel.description = getString(R.string.attention_channel_description)
        manager.createNotificationChannel(channel)
    }

    private companion object {
        const val ATTENTION_CHANNEL = "attention"
        const val PRIVACY_CHANNEL = "dev.ccm.orrery.remote/privacy"
    }
}
