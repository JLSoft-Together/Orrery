package dev.ccm.orrery.remote

import android.app.AlarmManager
import android.app.PendingIntent
import android.appwidget.AppWidgetManager
import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.os.Build
import android.service.quicksettings.TileService
import org.json.JSONArray
import org.json.JSONException

data class WaitingState(
    val count: Int,
    val labels: List<String>,
    val at: Long,
    val pausedUntil: Long,
) {
    fun hasData(): Boolean = count >= 0 && at > 0

    fun isFresh(now: Long): Boolean = hasData() && at <= now && now - at <= WaitingStore.STALE_MS

    fun isPaused(now: Long): Boolean = pausedUntil > now
}

object WaitingStore {
    const val STALE_MS = 30L * 60L * 1000L
    const val LABEL_MAX = 24
    const val LABEL_COUNT = 2
    const val COUNT_MAX = 999
    const val ACTION_REFRESH = "dev.ccm.orrery.remote.WAITING_REFRESH"

    private const val PREFS = "orrery_waiting"
    private const val KEY_COUNT = "count"
    private const val KEY_LABELS = "labels"
    private const val KEY_AT = "at"
    private const val KEY_PAUSED = "pausedUntil"
    private const val ALARM_CODE = 7113

    fun read(context: Context): WaitingState {
        val prefs = context.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
        val labels = prefs.getString(KEY_LABELS, "").orEmpty()
            .split('\n')
            .filter { it.isNotEmpty() }
        return WaitingState(
            count = prefs.getInt(KEY_COUNT, -1),
            labels = labels,
            at = prefs.getLong(KEY_AT, 0L),
            pausedUntil = prefs.getLong(KEY_PAUSED, 0L),
        )
    }

    fun apply(context: Context, count: Int, labels: List<String>, at: Long) {
        val prefs = context.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
        val current = read(context)
        if (current.at > at) return
        val clean = if (count > 0) labels.map(::cut).filter { it.isNotEmpty() }.take(LABEL_COUNT) else emptyList()
        prefs.edit()
            .putInt(KEY_COUNT, count.coerceIn(0, COUNT_MAX))
            .putString(KEY_LABELS, clean.joinToString("\n"))
            .putLong(KEY_AT, at)
            .apply()
        refresh(context)
    }

    fun setPausedUntil(context: Context, until: Long) {
        context.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
            .edit()
            .putLong(KEY_PAUSED, if (until > 0L) until else 0L)
            .apply()
        refresh(context)
    }

    fun applyPush(context: Context, w: String?, wl: String?, kind: String?, now: Long) {
        if (kind == "clear") {
            apply(context, 0, emptyList(), now)
            return
        }
        val raw = w?.trim() ?: return
        val count = raw.toIntOrNull() ?: return
        if (count < 0) return
        apply(context, count, parseLabels(wl), now)
    }

    fun parseLabels(raw: String?): List<String> {
        if (raw.isNullOrBlank()) return emptyList()
        val items = try {
            val array = JSONArray(raw)
            (0 until array.length()).mapNotNull { array.optString(it, "").takeIf { s -> s.isNotBlank() } }
        } catch (error: JSONException) {
            raw.split('\n', '\r')
        }
        return items.map { it.trim() }.filter { it.isNotEmpty() }.map(::cut).take(LABEL_COUNT)
    }

    fun refresh(context: Context) {
        val app = context.applicationContext
        val manager = AppWidgetManager.getInstance(app)
        val provider = ComponentName(app, OrreryWidgetProvider::class.java)
        val ids = manager.getAppWidgetIds(provider)
        if (ids.isNotEmpty()) OrreryWidgetProvider.render(app, manager, ids)
        try {
            TileService.requestListeningState(app, ComponentName(app, OrreryTileService::class.java))
        } catch (error: RuntimeException) {
            return
        } finally {
            scheduleRefresh(app)
        }
    }

    private fun scheduleRefresh(context: Context) {
        val alarms = context.getSystemService(AlarmManager::class.java) ?: return
        val intent = Intent(context, OrreryWidgetProvider::class.java).setAction(ACTION_REFRESH)
        val flags = PendingIntent.FLAG_UPDATE_CURRENT or
            (if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) PendingIntent.FLAG_IMMUTABLE else 0)
        val pending = PendingIntent.getBroadcast(context, ALARM_CODE, intent, flags)
        val now = System.currentTimeMillis()
        val state = read(context)
        val times = ArrayList<Long>()
        if (state.hasData() && state.at <= now) times.add(state.at + STALE_MS + 1000L)
        if (state.isPaused(now)) times.add(state.pausedUntil + 1000L)
        val next = times.filter { it > now }.minOrNull()
        if (next == null) {
            alarms.cancel(pending)
        } else {
            alarms.set(AlarmManager.RTC, next, pending)
        }
    }

    private fun cut(text: String): String {
        val trimmed = text.trim()
        val points = trimmed.codePointCount(0, trimmed.length)
        if (points <= LABEL_MAX) return trimmed
        return trimmed.substring(0, trimmed.offsetByCodePoints(0, LABEL_MAX))
    }
}
