import java.util.Properties
import java.io.FileInputStream

plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
    id("rust")
}

/* -------------------------
   Load Tauri properties
   ------------------------- */
val tauriProperties = Properties().apply {
    val propFile = file("tauri.properties")
    if (propFile.exists()) {
        propFile.inputStream().use { load(it) }
    }
}

/* -------------------------
   Load keystore properties
   ------------------------- */
val keystoreProperties = Properties().apply {
    val ksFile = rootProject.file("app/keystore.properties")
    if (ksFile.exists()) {
        FileInputStream(ksFile).use { load(it) }
    }
}

android {
    namespace = "com.zeus.roms_tauri"
    compileSdk = 36

    defaultConfig {
        applicationId = "com.zeus.roms_tauri"
        minSdk = 24
        targetSdk = 36

        versionCode = tauriProperties
            .getProperty("tauri.android.versionCode", "1")
            .toInt()

        versionName = tauriProperties
            .getProperty("tauri.android.versionName", "1.0")

        manifestPlaceholders["usesCleartextTraffic"] = "false"
    }

    /* -------------------------
       Signing configs
       ------------------------- */
    signingConfigs {
        create("release") {
            val storeFilePath =
                System.getenv("ROMSTAURI_KEYSTORE_PATH")
                    ?: keystoreProperties.getProperty("storeFile")

            if (!storeFilePath.isNullOrBlank()) {
                storeFile = file(storeFilePath)
            }

            storePassword =
                System.getenv("ROMSTAURI_KEYSTORE_PASSWORD")
                    ?: keystoreProperties.getProperty("storePassword")

            keyAlias =
                System.getenv("ROMSTAURI_KEY_ALIAS")
                    ?: keystoreProperties.getProperty("keyAlias")

            keyPassword =
                System.getenv("ROMSTAURI_KEY_PASSWORD")
                    ?: keystoreProperties.getProperty("keyPassword")
        }
    }

    buildTypes {
        getByName("debug") {
            isDebuggable = true
            isJniDebuggable = true
            isMinifyEnabled = false

            manifestPlaceholders["usesCleartextTraffic"] = "true"

            packaging {
                jniLibs.keepDebugSymbols.add("*/arm64-v8a/*.so")
                jniLibs.keepDebugSymbols.add("*/armeabi-v7a/*.so")
                jniLibs.keepDebugSymbols.add("*/x86/*.so")
                jniLibs.keepDebugSymbols.add("*/x86_64/*.so")
            }
        }

        getByName("release") {
            signingConfig = signingConfigs.getByName("release")
            isMinifyEnabled = true
            isShrinkResources = false

            proguardFiles(
                *fileTree(".") { include("**/*.pro") }
                    .plus(getDefaultProguardFile("proguard-android-optimize.txt"))
                    .toList()
                    .toTypedArray()
            )
        }
    }

    kotlinOptions {
        jvmTarget = "1.8"
    }

    buildFeatures {
        buildConfig = true
    }
}

/* -------------------------
   Rust config (Tauri)
   ------------------------- */
rust {
    rootDirRel = "../../../"
}

/* -------------------------
   Dependencies
   ------------------------- */
dependencies {
    implementation("androidx.webkit:webkit:1.14.0")
    implementation("androidx.appcompat:appcompat:1.7.1")
    implementation("androidx.activity:activity-ktx:1.10.1")
    implementation("com.google.android.material:material:1.12.0")

    testImplementation("junit:junit:4.13.2")
    androidTestImplementation("androidx.test.ext:junit:1.1.4")
    androidTestImplementation("androidx.test.espresso:espresso-core:3.5.0")
}

/* -------------------------
   Tauri build glue
   ------------------------- */
apply(from = "tauri.build.gradle.kts")
