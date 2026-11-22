import java.util.Locale

plugins {
    kotlin("jvm") version "2.3.0"
    application
}

val paparazziVersion = "2.0.0-alpha05.1"
// Must match the layoutlib version the Paparazzi release was compiled against (see its POM).
val layoutlibVersion = "16.2.3"

repositories {
    google()
    mavenCentral()
}

val osLabel: String = System.getProperty("os.name").lowercase(Locale.US).let { os ->
    when {
        os.startsWith("windows") -> "win"
        os.startsWith("mac") -> if (System.getProperty("os.arch").startsWith("x86")) "mac" else "mac-arm"
        else -> "linux"
    }
}

val layoutlibRuntime by configurations.creating { isTransitive = false }
val layoutlibResources by configurations.creating { isTransitive = false }

dependencies {
    implementation("app.cash.paparazzi:paparazzi:$paparazziVersion")
    implementation("com.android.tools.layoutlib:layoutlib:$layoutlibVersion")
    layoutlibRuntime("com.android.tools.layoutlib:layoutlib-runtime:$layoutlibVersion:$osLabel")
    layoutlibResources("com.android.tools.layoutlib:layoutlib-resources:$layoutlibVersion")
}

kotlin { jvmToolchain(21) }

val unpackRuntime by tasks.registering(Sync::class) {
    from({ layoutlibRuntime.map { zipTree(it) } })
    into(layout.buildDirectory.dir("layoutlib/runtime"))
}
val unpackResources by tasks.registering(Sync::class) {
    from({ layoutlibResources.map { zipTree(it) } })
    into(layout.buildDirectory.dir("layoutlib/resources"))
}

application { mainClass.set("spike.MainKt") }

tasks.named<JavaExec>("run") {
    dependsOn(unpackRuntime, unpackResources)
    workingDir = projectDir
    jvmArgs("-Xmx1g", "-XX:+EnableDynamicAgentLoading", "-Djdk.attach.allowAttachSelf=true")
    systemProperty("paparazzi.layoutlib.runtime.root", layout.buildDirectory.dir("layoutlib/runtime").get().asFile.path)
    systemProperty("paparazzi.layoutlib.resources.root", layout.buildDirectory.dir("layoutlib/resources").get().asFile.path)
    args = listOfNotNull(project.findProperty("xml")?.toString())
}
