package dev.ccm.orrery.remote

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent

class WaitingPushReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        val extras = intent.extras ?: return
        val kind = extras.getString("k")
        val count = extras.getString("w")
        if (kind != "clear" && count == null) return
        WaitingStore.applyPush(context, count, extras.getString("wl"), kind, System.currentTimeMillis())
    }
}
