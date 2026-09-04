/* Helpers for file operations */

package info.scoria

import android.app.Activity
import android.content.ContentValues
import android.content.Intent
import android.net.Uri
import android.os.Environment
import android.os.Handler
import android.os.Looper
import android.provider.MediaStore
import android.util.Log
import androidx.core.content.FileProvider
import java.io.File
import java.io.FileInputStream
import java.io.FileNotFoundException
import java.io.FileOutputStream
import java.nio.file.Path
import java.time.LocalDateTime
import java.time.format.DateTimeFormatter
import kotlin.io.path.Path
import kotlin.io.path.copyTo

val TAG = "ScoriaFiles"
val SHARE_CODE = 5 // identifies this sharing action (our choice)
val IMPORT_SQLITE_CODE = 6 // identifies the import sqlite action (our choice)
val IMPORT_PLACES_GEOJSON_CODE = 7 // same as above, but for places
val IMPORT_MOUNTED_DB_CODE = 8 // for mounting a database
val SHARE_DEL_FILE_DELAY: Long = 10000 // how long after share action to wait
                                       // before deleting the file
val DB_EXPORT_PREFIX = "Scoria_Export_"
val TRACK_EXPORT_PREFIX = "Scoria_Track_"
val TRACK_TEMP_PREFIX = "track_export"
val IMAGE_EXPORT_PREFIX = "Scoria_Image_"
val IMAGE_TEMP_NAME = "image_export.jpeg"
val DB_IMPORT_NAME = "scoria_import.db"
val PLACES_GEOJSON_IMPORT_NAME = "places.geojson"
val MOUNTED_DB_IMPORT_NAME_FALLBACK = "import.db"
val MIME_TYPE = "*/*" // Not all file manager apps seem to accept any mime
                      // type. Ideally we'd use "application/x-sqlite3", but
                      // "*/*" has the most compatibility

class ScoriaFileProvider : FileProvider() {
    companion object {
        // static member for tracking the temporary files to clean after
        // sharing
        var sharedFilesToCleanup: List<Path> = emptyList()
    }
}

public fun getDbExportName(): String {
    return DB_EXPORT_PREFIX + getFormattedDateTime() + ".sqlite"
}

// Get the new track export name without an extension
public fun getTrackExportName(): String {
    return TRACK_EXPORT_PREFIX + getFormattedDateTime()
}

// Get the new image export name
public fun getImageExportName(): String {
    return IMAGE_EXPORT_PREFIX + getFormattedDateTime() + ".jpeg"
}

public fun getFormattedDateTime(): String {
    val now = LocalDateTime.now()
    val formatter = DateTimeFormatter.ofPattern("yyyy-MM-dd_HH-mm-ss")
    val formattedDateTime = now.format(formatter)
    return formattedDateTime
}

// Copy files into the cache dir with new final path components, then share
// them. `extraFiles` are (source, new name) pairs shared alongside `source`.
public fun shareWithName(
    activity: Activity,
    source: Path,
    newFname: String,
    extraFiles: List<Pair<Path, String>> = emptyList(),
) {
    val cacheDir = Path(activity.getCacheDir().getAbsolutePath())
    val renamed = mutableListOf<Path>()
    try {
        for ((src, name) in listOf(source to newFname) + extraFiles) {
            val dst = cacheDir.resolve(name)
            src.copyTo(dst, true)
            renamed.add(dst)
        }
    } catch (e: Exception) {
        Stem.logError("Failed to rename file during export: ${e}")
        return
    }
    shareFiles(activity, renamed)
}

// Share a file that is already in the cache dir with the correct name
public fun shareFile(activity: Activity, path: Path, mimeType: String = MIME_TYPE) {
    shareFiles(activity, listOf(path), mimeType)
}

// Share files that are already in the cache dir with the correct names
public fun shareFiles(activity: Activity, paths: List<Path>, mimeType: String = MIME_TYPE) {
    val contentUris = ArrayList(paths.map { path ->
        FileProvider.getUriForFile(
            activity,
            "info.scoria.fileprovider",
            path.toFile()
        )
    })
    val sendIntent: Intent = if (contentUris.size == 1) {
        Intent().apply {
            action = Intent.ACTION_SEND
            putExtra(Intent.EXTRA_STREAM, contentUris[0])
            type = mimeType
        }
    } else {
        Intent().apply {
            action = Intent.ACTION_SEND_MULTIPLE
            putParcelableArrayListExtra(Intent.EXTRA_STREAM, contentUris)
            type = mimeType
        }
    }
    sendIntent.addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
    // create a chooser around the intent to let the user pick how to open it
    val shareIntent = Intent.createChooser(sendIntent, "Save to:")
    activity.startActivityForResult(shareIntent, SHARE_CODE)

    ScoriaFileProvider.sharedFilesToCleanup = paths
}

// Initiate a file request. The result is handled in `onActivityResult`.
//
// Valid request codes are IMPORT_SQLITE_CODE, IMPORT_PLACES_GEOJSON_CODE, etc.
public fun initiateImport(activity: Activity, code: Int) {
    val requestFileIntent = Intent(Intent.ACTION_GET_CONTENT).apply {
        type = MIME_TYPE
        addCategory(Intent.CATEGORY_OPENABLE)
    }
    activity.startActivityForResult(requestFileIntent, code)
}

public fun completeSqliteImport(activity: Activity, uri: Uri) {
    readURIToFile(activity, uri, DB_IMPORT_NAME)?.also { tempFile ->
        Stem.importFromSqliteLog(tempFile.absolutePath);
        tempFile.delete();
    }
}

public fun completePlacesGeojsonImport(activity: Activity, uri: Uri) {
    readURIToFile(activity, uri, PLACES_GEOJSON_IMPORT_NAME)?.also { tempFile ->
        Stem.importFromPlacesGeojson(tempFile.absolutePath);
        tempFile.delete();
    }
}

public fun completeMountedDBImport(activity: Activity, uri: Uri) {
    val fname = uri.getLastPathSegment() ?: MOUNTED_DB_IMPORT_NAME_FALLBACK;
    readURIToFile(activity, uri, fname)?.also { tempFile ->
        Stem.importMountedDB(tempFile.absolutePath);
        tempFile.delete();
    }
}

// Have a possibly valid file URI, try to convert to a real file on disk
// Ref: https://developer.android.com/training/secure-file-sharing/request-file
public fun readURIToFile(activity: Activity, uri: Uri, fname: String): File? {
    // Try to open the file for "read" access using the
    // returned URI. If the file isn't found, write to the
    // error log and return.
    val inputPFD = try {
        // Get the content resolver instance for this context, and use it
        // to get a ParcelFileDescriptor for the file.
        activity.contentResolver.openFileDescriptor(uri, "r")
    } catch (e: FileNotFoundException) {
        e.printStackTrace()
        Log.e(TAG, "File not found.")
        return null
    }
    // Get a regular file descriptor for the file
    // val fd = inputPFD.fileDescriptor
    return inputPFD?.fileDescriptor.let {
        val inputStream = FileInputStream(it)
        val tempFile = Path(activity.getCacheDir().getAbsolutePath())
            .resolve(fname).toFile()
        val outputStream = FileOutputStream(tempFile)
        val buffer = ByteArray(1024)
        var length: Int
        while (inputStream.read(buffer).also { length = it } > 0) {
            outputStream.write(buffer, 0, length)
        }
        outputStream.close()
        inputStream.close()
        tempFile
    }
}

public fun exportTrack(activity: Activity) {
    val dir = activity.getCacheDir()
    val files = dir.listFiles()
    if (files != null) {
        files.forEach { file ->
            if (file.name.startsWith(TRACK_TEMP_PREFIX)) {
                val ext = file.extension
                val newName = getTrackExportName()
                val renamed = File(file.parent, "$newName.$ext")

                val isRenamed = file.renameTo(renamed)
                if (!isRenamed) {
                    Stem.logError("Failed to rename export file")
                }
                shareFile(activity, renamed.toPath())
                return
            }
        }
    }
}

public fun exportImage(activity: Activity) {
    val dir = activity.getCacheDir()
    val file = File(dir, IMAGE_TEMP_NAME)
    val renamed = File(file.parent, getImageExportName())
    val isRenamed = file.renameTo(renamed)
    Log.i(TAG, "src: $file, renamed: $renamed")
    if (!isRenamed) {
        Stem.logError("Failed to rename export image file")
        return
    }
    saveImageToGallery(activity, renamed)
    shareFile(activity, renamed.toPath(), "image/jpeg")
}

// Persist an image into the media store (Pictures/Scoria) so it shows up in
// gallery apps. Captures persist like screenshots do; the share sheet is only
// for sending the image onward. No permission is needed for an app to insert
// its own images.
public fun saveImageToGallery(activity: Activity, file: File) {
    val values = ContentValues().apply {
        put(MediaStore.Images.Media.DISPLAY_NAME, file.name)
        put(MediaStore.Images.Media.MIME_TYPE, "image/jpeg")
        put(
            MediaStore.Images.Media.RELATIVE_PATH,
            Environment.DIRECTORY_PICTURES + "/Scoria"
        )
        // hide the entry from other apps until the bytes are written
        put(MediaStore.Images.Media.IS_PENDING, 1)
    }
    val resolver = activity.contentResolver
    val uri = resolver.insert(
        MediaStore.Images.Media.EXTERNAL_CONTENT_URI, values
    )
    if (uri == null) {
        Stem.logError("Failed to create gallery entry for exported image")
        return
    }
    try {
        resolver.openOutputStream(uri)?.use { out ->
            file.inputStream().use { it.copyTo(out) }
        } ?: throw FileNotFoundException("null output stream for $uri")
        values.clear()
        values.put(MediaStore.Images.Media.IS_PENDING, 0)
        resolver.update(uri, values, null, null)
    } catch (e: Exception) {
        Stem.logError("Failed to save exported image to gallery: ${e}")
        resolver.delete(uri, null, null)
    }
}

// Upon starting of the app we 
public fun cleanupSharedFile() {
    val handler = Handler(Looper.getMainLooper())
    // give the receiving app 10 seconds to copy the file before deleting it
    handler.postDelayed({
        ScoriaFileProvider.sharedFilesToCleanup.forEach { path ->
            val isDeleted = path.toFile().delete()
            if (!isDeleted) {
                Stem.logError("Failed to delete temp file after export")
            }
        }
        ScoriaFileProvider.sharedFilesToCleanup = emptyList()
    }, SHARE_DEL_FILE_DELAY)
}

// Cleanup shared files matching a the prefixes that are used by Scoria. This is
// useful in the case the app crashes during a share event, and fails to receive
// the activity result. This is run less often, such as on app startup.
public fun cleanupAllSharedFiles(activity: Activity) {
    val dir = activity.getCacheDir()
    val prefixesToRemove = listOf(
        DB_EXPORT_PREFIX,
        TRACK_EXPORT_PREFIX,
        TRACK_TEMP_PREFIX,
        DB_IMPORT_NAME,
        IMAGE_TEMP_NAME,
        IMAGE_EXPORT_PREFIX,
    )

    val files = dir.listFiles()
    if (files != null) {
        val filesToRemove = files.filter { file ->
            file.isFile && prefixesToRemove.any { prefix ->
                file.name.startsWith(prefix)
            }
        }

        filesToRemove.forEach { file ->
            file.delete()
        }
    }
}
