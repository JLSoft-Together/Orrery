package spike

import android.view.View
import android.view.ViewGroup
import android.widget.FrameLayout
import app.cash.paparazzi.DeviceConfig
import app.cash.paparazzi.Environment
import app.cash.paparazzi.PaparazziSdk
import com.android.ide.common.rendering.api.ILayoutPullParser
import com.android.ide.common.rendering.api.ResourceNamespace
import com.android.layoutlib.bridge.android.BridgeContext
import com.android.layoutlib.bridge.android.BridgeXmlBlockParser
import org.kxml2.io.KXmlParser
import org.xmlpull.v1.XmlPullParser
import java.awt.image.BufferedImage
import java.io.File
import java.io.StringReader
import javax.imageio.ImageIO

/** Cookie = 1-based line of the start tag, so a rendered view maps back to its XML line. */
class StringLayoutParser(xml: String) : KXmlParser(), ILayoutPullParser {
    init {
        setFeature(XmlPullParser.FEATURE_PROCESS_NAMESPACES, true)
        setInput(StringReader(xml))
    }

    override fun getViewCookie(): Any? = if (eventType == XmlPullParser.START_TAG) lineNumber else null

    override fun getLayoutNamespace(): ResourceNamespace = ResourceNamespace.RES_AUTO
}

private fun ms(fromNanos: Long) = (System.nanoTime() - fromNanos) / 1_000_000.0

fun main(args: Array<String>) {
    val xmlFile = File(args.getOrNull(0) ?: "sample/sample.xml")
    val renders = args.getOrNull(1)?.toInt() ?: 10
    val outDir = File("out").apply { mkdirs() }
    val xml = xmlFile.readText()

    val t0 = System.nanoTime()
    var frame: BufferedImage? = null
    val sdk = PaparazziSdk(
        environment = Environment(
            appTestDir = outDir.absolutePath,
            packageName = "spike",
            compileSdkVersion = 36,
            resourcePackageNames = emptyList(),
            localResourceDirs = emptyList(),
            moduleResourceDirs = emptyList(),
            libraryResourceDirs = emptyList(),
            allModuleAssetDirs = emptyList(),
            libraryAssetDirs = emptyList(),
        ),
        deviceConfig = DeviceConfig.PIXEL_5,
        theme = "android:Theme.Material.Light.NoActionBar",
        useDeviceResolution = true,
        onNewFrame = { frame = it },
    )
    sdk.setup()
    sdk.prepare()
    val initMs = ms(t0)

    fun render(): BufferedImage {
        val ctx = sdk.context as BridgeContext
        val parser = BridgeXmlBlockParser(StringLayoutParser(xml), ctx, ResourceNamespace.RES_AUTO)
        val view = try {
            sdk.layoutInflater.inflate(parser, FrameLayout(ctx), false)
        } finally {
            parser.ensurePopped()
        }
        sdk.snapshot(view)
        lastTree = dumpTree(ctx, view, 0)
        return frame ?: error("no frame produced")
    }

    val t1 = System.nanoTime()
    val first = render()
    val firstMs = ms(t1)
    val coldMs = ms(t0)
    val jvmUptimeMs = java.lang.management.ManagementFactory.getRuntimeMXBean().uptime

    val t2 = System.nanoTime()
    val png = File(outDir, "sample.png")
    ImageIO.write(first, "png", png)
    val pngMs = ms(t2)

    val times = (1..renders).map {
        val t = System.nanoTime()
        render()
        ms(t)
    }
    sdk.teardown()

    println("view tree (class @line [l,t,r,b]):")
    print(lastTree)
    println("png: ${png.absolutePath} (${first.width}x${first.height})")
    println("java: ${System.getProperty("java.vendor")} ${System.getProperty("java.version")}")
    println("init (setup+prepare) ms: %.0f".format(initMs))
    println("first render ms: %.0f".format(firstMs))
    println("cold start to first frame ms: %.0f".format(coldMs))
    println("jvm start to first frame ms: $jvmUptimeMs")
    println("png write ms: %.0f".format(pngMs))
    println("warm renders x$renders ms: avg %.1f, min %.1f, max %.1f".format(times.average(), times.min(), times.max()))
}

private var lastTree = ""

private fun dumpTree(ctx: BridgeContext, view: View, depth: Int): String = buildString {
    // getLocationInWindow is not reliable in layoutlib, so bounds are accumulated from left/top.
    val loc = absolutePos(view)
    append("  ".repeat(depth))
    append("${view.javaClass.simpleName} @${ctx.getViewKey(view)} [${loc.first},${loc.second},${loc.first + view.width},${loc.second + view.height}]\n")
    if (view is ViewGroup) for (i in 0 until view.childCount) append(dumpTree(ctx, view.getChildAt(i), depth + 1))
}

private fun absolutePos(view: View): Pair<Int, Int> {
    var x = 0
    var y = 0
    var v: View? = view
    while (v != null) {
        x += v.left
        y += v.top
        v = v.parent as? View
    }
    return x to y
}
