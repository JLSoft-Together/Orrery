package dev.ccm.orrery.remote

import android.app.NotificationChannel
import android.app.NotificationManager
import android.os.Build
import android.os.Bundle
import io.flutter.embedding.android.FlutterActivity

class MainActivity : FlutterActivity() {
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
    }
}
