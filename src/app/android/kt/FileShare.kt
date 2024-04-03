/* Helpers for file operations */

package info.scoria

import android.app.Activity
import android.content.Intent
import android.net.Uri
import android.os.Handler
import android.os.Looper
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
val IMPORT_CODE = 6 // identifies the import action (our choice)
val SHARE_DEL_FILE_DELAY: Long = 10000 // how long after share action to wait
                                       // before deleting the file
val DB_EXPORT_PREFIX = "Scoria_Export_"
val TRACK_EXPORT_PREFIX = "Scoria_Track_"
val TRACK_TEMP_PREFIX = "track_export"
val DB_IMPORT_NAME = "scoria_import.db"
val MIME_TYPE = "*/*" // Not all file manager apps seem to accept any mime
                      // type. Ideally we'd use "application/x-sqlite3", but
                      // "*/*" has the most compatibility

class ScoriaFileProvider : FileProvider() {
    companion object {
        // static member for tracking the temporary file to clean after sharing
        var sharedFileToCleanup: Path? = null
    }
}

public fun getDbExportName(): String {
    return DB_EXPORT_PREFIX + getFormattedDateTime() + ".sqlite"
}

// Get the new track export name without an extension
public fun getTrackExportName(): String {
    return TRACK_EXPORT_PREFIX + getFormattedDateTime()
}

public fun getFormattedDateTime(): String {
    val now = LocalDateTime.now()
    val formatter = DateTimeFormatter.ofPattern("yyyy-MM-dd_HH-mm-ss")
    val formattedDateTime = now.format(formatter)
    return formattedDateTime
}

// Rename a file from `source` into the cache dir with new final path component
// `newFname`, then share it
public fun shareWithName(
    activity: Activity,
    source: Path,
    newFname: String,
) {
    val renamed =
        Path(activity.getCacheDir().getAbsolutePath()).resolve(newFname)
    try {
        source.copyTo(renamed, true)
    } catch (e: Exception) {
        Stem.logError("Failed to rename file during export: ${e}")
        return
    }
    shareFile(activity, renamed)
}

// Share a file that is already in the cache dir with the correct name
public fun shareFile(activity: Activity, path: Path) {
    val contentUri = FileProvider.getUriForFile(
        activity, 
        "info.scoria.fileprovider", 
        path.toFile()
    )
    val sendIntent: Intent = Intent().apply {
        action = Intent.ACTION_SEND
        putExtra(Intent.EXTRA_STREAM, contentUri)
        type = MIME_TYPE
    }
    sendIntent.addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
    // create a chooser around the intent to let the user pick how to open it
    val shareIntent = Intent.createChooser(sendIntent, "Save your log to:")
    activity.startActivityForResult(shareIntent, SHARE_CODE)

    ScoriaFileProvider.sharedFileToCleanup = path
}

// Initiate a file request. The result is handled in `onActivityResult`.
public fun initiateImport(activity: Activity) {
    val requestFileIntent = Intent(Intent.ACTION_GET_CONTENT).apply {
        type = MIME_TYPE
        addCategory(Intent.CATEGORY_OPENABLE)
    }
    activity.startActivityForResult(requestFileIntent, IMPORT_CODE)
}

// Have a possibly valid file URI, try to import it into Scoria
// Ref: https://developer.android.com/training/secure-file-sharing/request-file
public fun completeImport(activity: Activity, uri: Uri) {
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
        return
    }
    // Get a regular file descriptor for the file
    // val fd = inputPFD.fileDescriptor
    inputPFD?.fileDescriptor.also { fd ->
        val inputStream = FileInputStream(fd)
        val tempFile = Path(activity.getCacheDir().getAbsolutePath())
            .resolve(DB_IMPORT_NAME).toFile()
        val outputStream = FileOutputStream(tempFile)
        val buffer = ByteArray(1024)
        var length: Int
        while (inputStream.read(buffer).also { length = it } > 0) {
            outputStream.write(buffer, 0, length)
        }
        outputStream.close()
        inputStream.close()
        Stem.importFromSqliteLog(tempFile.absolutePath)
        tempFile.delete()
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

// Upon starting of the app we 
public fun cleanupSharedFile() {
    val handler = Handler(Looper.getMainLooper())
    // give the receiving app 10 seconds to copy the file before deleting it
    handler.postDelayed({
        ScoriaFileProvider.sharedFileToCleanup?.also { path ->
            val isDeleted = path.toFile().delete()
            if (!isDeleted) {
                Stem.logError("Failed to delete temp file after export")
            }
        }
        ScoriaFileProvider.sharedFileToCleanup = null
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
        DB_IMPORT_NAME
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
