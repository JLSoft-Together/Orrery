package dev.ccm.orrery.remote

import android.app.Activity
import android.content.ActivityNotFoundException
import android.content.Intent
import android.net.Uri
import android.os.Build
import android.os.StatFs
import android.provider.Settings
import androidx.core.content.FileProvider
import io.flutter.plugin.common.BinaryMessenger
import io.flutter.plugin.common.MethodChannel
import java.io.File

object ApkInstaller {
    const val CHANNEL = "dev.ccm.orrery.remote/install"
    private const val DIR = "apk_offer"
    private const val AUTHORITY_SUFFIX = ".apkprovider"
    private const val MIME = "application/vnd.android.package-archive"

    fun register(activity: Activity, messenger: BinaryMessenger) {
        MethodChannel(messenger, CHANNEL).setMethodCallHandler { call, result ->
            when (call.method) {
                "canInstall" -> result.success(canInstall(activity))
                "openSettings" -> result.success(openSettings(activity))
                "freeBytes" -> result.success(freeBytes(activity, call.argument<String>("path")))
                "install" -> result.success(install(activity, call.argument<String>("path")))
                else -> result.notImplemented()
            }
        }
    }

    private fun canInstall(activity: Activity): Boolean =
        Build.VERSION.SDK_INT < Build.VERSION_CODES.O ||
            activity.packageManager.canRequestPackageInstalls()

    private fun openSettings(activity: Activity): Boolean {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.O) return false
        return try {
            activity.startActivity(
                Intent(Settings.ACTION_MANAGE_UNKNOWN_APP_SOURCES, Uri.parse("package:" + activity.packageName)),
            )
            true
        } catch (error: ActivityNotFoundException) {
            false
        }
    }

    private fun apkDir(activity: Activity): File = File(activity.cacheDir, DIR)

    private fun inside(activity: Activity, path: String?): File? {
        if (path.isNullOrEmpty()) return null
        return try {
            val file = File(path).canonicalFile
            val root = apkDir(activity).canonicalFile
            if (file.parentFile == root && file.isFile && file.name.endsWith(".apk")) file else null
        } catch (error: java.io.IOException) {
            null
        }
    }

    private fun freeBytes(activity: Activity, path: String?): Long? {
        val dir = apkDir(activity)
        dir.mkdirs()
        return try {
            StatFs(dir.absolutePath).availableBytes
        } catch (error: IllegalArgumentException) {
            null
        }
    }

    private fun install(activity: Activity, path: String?): String {
        if (!canInstall(activity)) return "notAllowed"
        val file = inside(activity, path) ?: return "failed"
        return try {
            val uri = FileProvider.getUriForFile(activity, activity.packageName + AUTHORITY_SUFFIX, file)
            val intent = Intent(Intent.ACTION_VIEW)
                .setDataAndType(uri, MIME)
                .addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_ACTIVITY_NEW_TASK)
            activity.startActivity(intent)
            "started"
        } catch (error: ActivityNotFoundException) {
            "failed"
        } catch (error: IllegalArgumentException) {
            "failed"
        } catch (error: SecurityException) {
            "failed"
        }
    }
}
