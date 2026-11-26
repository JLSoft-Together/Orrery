package dev.ccm.orrery.remote

import android.app.PendingIntent
import android.content.Intent
import android.graphics.drawable.Icon
import android.os.Build
import android.service.quicksettings.Tile
import android.service.quicksettings.TileService

class OrreryTileService : TileService() {
    override fun onStartListening() {
        super.onStartListening()
        update()
    }

    override fun onTileAdded() {
        super.onTileAdded()
        update()
    }

    override fun onClick() {
        super.onClick()
        val intent = Intent(this, MainActivity::class.java)
            .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TOP)
        if (Build.VERSION.SDK_INT >= 34) {
            val pending = PendingIntent.getActivity(this, 0, intent, PendingIntent.FLAG_IMMUTABLE)
            startActivityAndCollapse(pending)
        } else {
            collapseLegacy(intent)
        }
    }

    @Suppress("DEPRECATION")
    private fun collapseLegacy(intent: Intent) {
        startActivityAndCollapse(intent)
    }

    private fun update() {
        val tile = qsTile ?: return
        val now = System.currentTimeMillis()
        val state = WaitingStore.read(this)
        val subtitle: String
        var active = false
        if (state.isPaused(now)) {
            subtitle = getString(R.string.waiting_paused_until, OrreryWidgetProvider.formatTime(state.pausedUntil))
        } else if (state.isFresh(now)) {
            subtitle = getString(R.string.tile_waiting_count, state.count)
            active = state.count > 0
        } else {
            subtitle = getString(R.string.waiting_open_app)
        }
        val name = getString(R.string.tile_label)
        tile.icon = Icon.createWithResource(this, R.drawable.ic_stat_orrery)
        tile.state = if (active) Tile.STATE_ACTIVE else Tile.STATE_INACTIVE
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
            tile.label = name
            tile.subtitle = subtitle
        } else {
            tile.label = "$name, $subtitle"
        }
        tile.updateTile()
    }
}
