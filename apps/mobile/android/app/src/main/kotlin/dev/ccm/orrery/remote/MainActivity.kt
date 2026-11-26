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
    private var pairChannel: MethodChannel? = null
    private var pairPending: Map<String, Any>? = null
    private var pairReady = false
    private var waitingChannel: MethodChannel? = null
    private var waitingReady = false
    private var pausePending = false

    override fun configureFlutterEngine(flutterEngine: FlutterEngine) {
        super.configureFlutterEngine(flutterEngine)
        pairReady = false
        pairChannel = MethodChannel(flutterEngine.dartExecutor.binaryMessenger, PAIR_CHANNEL).also { channel ->
            channel.setMethodCallHandler { call, result ->
                when (call.method) {
                    "ready" -> {
                        pairReady = true
                        val pending = pairPending
                        pairPending = null
                        result.success(pending)
                    }
                    else -> result.notImplemented()
                }
            }
        }
        waitingReady = false
        waitingChannel = MethodChannel(flutterEngine.dartExecutor.binaryMessenger, WAITING_CHANNEL).also { channel ->
            channel.setMethodCallHandler { call, result ->
                when (call.method) {
                    "ready" -> {
                        waitingReady = true
                        val pending = pausePending
                        pausePending = false
                        result.success(pending)
                    }
                    "apply" -> {
                        val count = call.argument<Int>("count")
                        val at = call.argument<Number>("at")?.toLong()
                        val labels = call.argument<List<String>>("labels").orEmpty()
                        if (count == null || at == null) {
                            result.error("bad_args", null, null)
                        } else {
                            WaitingStore.apply(applicationContext, count, labels, at)
                            result.success(null)
                        }
                    }
                    "setPausedUntil" -> {
                        val until = call.argument<Number>("until")?.toLong() ?: 0L
                        WaitingStore.setPausedUntil(applicationContext, until)
                        result.success(null)
                    }
                    else -> result.notImplemented()
                }
            }
        }
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
        if (savedInstanceState == null) {
            takePairIntent(intent)
            takePauseIntent(intent)
        }
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        setIntent(intent)
        takePairIntent(intent)
        takePauseIntent(intent)
    }

    private fun takePauseIntent(intent: Intent?) {
        if (intent?.action != TILE_PREFERENCES_ACTION) return
        intent.action = Intent.ACTION_MAIN
        val channel = waitingChannel
        if (waitingReady && channel != null) {
            channel.invokeMethod("pause", null)
        } else {
            pausePending = true
        }
    }

    private fun takePairIntent(intent: Intent?) {
        if (intent == null) return
        val qr = intent.getStringExtra(PAIR_EXTRA)
        if (qr.isNullOrEmpty()) return
        intent.removeExtra(PAIR_EXTRA)
        val request = mapOf("qr" to qr, "fromShell" to isFromShell())
        val channel = pairChannel
        if (pairReady && channel != null) {
            channel.invokeMethod("pair", request)
        } else {
            pairPending = request
        }
    }

    private fun isFromShell(): Boolean =
        referrer?.host == SHELL_PACKAGE || callingPackage == SHELL_PACKAGE

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
        const val PAIR_CHANNEL = "dev.ccm.orrery.remote/pair"
        const val WAITING_CHANNEL = "dev.ccm.orrery.remote/waiting"
        const val TILE_PREFERENCES_ACTION = "android.service.quicksettings.action.QS_TILE_PREFERENCES"
        const val PAIR_EXTRA = "pairQr"
        const val SHELL_PACKAGE = "com.android.shell"
    }
}
