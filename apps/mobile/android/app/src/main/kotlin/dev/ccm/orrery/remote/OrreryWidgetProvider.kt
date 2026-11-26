package dev.ccm.orrery.remote

import android.app.PendingIntent
import android.appwidget.AppWidgetManager
import android.appwidget.AppWidgetProvider
import android.content.Context
import android.content.Intent
import android.os.Build
import android.view.View
import android.widget.RemoteViews
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale

class OrreryWidgetProvider : AppWidgetProvider() {
    override fun onUpdate(context: Context, manager: AppWidgetManager, appWidgetIds: IntArray) {
        render(context, manager, appWidgetIds)
        WaitingStore.refresh(context)
    }

    override fun onReceive(context: Context, intent: Intent) {
        if (intent.action == WaitingStore.ACTION_REFRESH) {
            WaitingStore.refresh(context)
            return
        }
        super.onReceive(context, intent)
    }

    companion object {
        fun render(context: Context, manager: AppWidgetManager, ids: IntArray) {
            val now = System.currentTimeMillis()
            val state = WaitingStore.read(context)
            for (id in ids) manager.updateAppWidget(id, views(context, state, now))
        }

        fun formatTime(at: Long): String = SimpleDateFormat("HH:mm", Locale.getDefault()).format(Date(at))

        private fun views(context: Context, state: WaitingState, now: Long): RemoteViews {
            val views = RemoteViews(context.packageName, R.layout.widget_waiting)
            val fresh = state.isFresh(now)
            if (fresh) {
                views.setViewVisibility(R.id.widget_count, View.VISIBLE)
                views.setTextViewText(R.id.widget_count, state.count.toString())
                val labels = if (state.count == 0) {
                    context.getString(R.string.widget_none)
                } else {
                    state.labels.joinToString("\n")
                }
                views.setTextViewText(R.id.widget_labels, labels)
                views.setTextViewText(R.id.widget_time, context.getString(R.string.widget_at, formatTime(state.at)))
                views.setViewVisibility(R.id.widget_time, View.VISIBLE)
            } else {
                views.setViewVisibility(R.id.widget_count, View.GONE)
                views.setTextViewText(R.id.widget_labels, context.getString(R.string.waiting_open_app))
                views.setViewVisibility(R.id.widget_time, View.GONE)
            }
            if (state.isPaused(now)) {
                views.setTextViewText(
                    R.id.widget_paused,
                    context.getString(R.string.waiting_paused_until, formatTime(state.pausedUntil)),
                )
                views.setViewVisibility(R.id.widget_paused, View.VISIBLE)
            } else {
                views.setViewVisibility(R.id.widget_paused, View.GONE)
            }
            views.setOnClickPendingIntent(R.id.widget_root, openApp(context))
            return views
        }

        private fun openApp(context: Context): PendingIntent {
            val intent = Intent(context, MainActivity::class.java)
                .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TOP)
            val flags = PendingIntent.FLAG_UPDATE_CURRENT or
                (if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) PendingIntent.FLAG_IMMUTABLE else 0)
            return PendingIntent.getActivity(context, 0, intent, flags)
        }
    }
}
