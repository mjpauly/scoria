import StemLib
import UIKit

// Shared type understanding by the Sensing and Top code regarding what the ViewController can do.
public protocol MyViewControllerProtocol: UIViewController, UIDocumentPickerDelegate {}

var myLocationManager = MyLocationManager()
public var server_port: UInt16 = 0;
public var server_scope: UInt64 = 0;

public func startup() {
    // Set the app directories known to the core library and get the
    // backend server port and secret key
    let app_version = Bundle.main.infoDictionary?["CFBundleShortVersionString"] as? String ?? "Unknown"
    set_app_dirs(getDocumentsDirectory().path(),
                 getLibraryDirectory().path(),
                 getTemporaryDirectoryPath(),
                 getBundlePath(),
                 app_version)
    myLocationManager.touch()  // initialize the lazy global var
}

// handle shutdown of the app by saving certain parts of the app state
public func app_shutdown() {
    handle_shutdown()
}

// handler for pokes that come from the frontend
public func handle_poke(viewController: MyViewControllerProtocol) {
    // load config from backend since we got poked by the frontend
    check_request_when_in_use_authorization()
    myLocationManager.updateConfig()
    check_export_sqlite_log(viewController: viewController)
    check_import_sqlite_log(viewController: viewController)
    check_export_track(viewController: viewController)
}

public func is_location_on() -> Bool {
    return myLocationManager.isOn()
}

// check if we should prompt for when-in-use authorization
func check_request_when_in_use_authorization() {
    if should_request_when_in_use_authorization() {
        myLocationManager.locationManager.requestWhenInUseAuthorization()
    }
}

public func print_and_log_error(s: String) {
    print(s)
    log_error(s)
}

public func handle_background() {
    // App went to background -> stop the UI server
    handle_enter_background()
}

public func handle_foreground() {
    // App coming to foreground -> restart the UI server
    let server_config = handle_enter_foreground()
    server_port = server_config.port
    server_scope = server_config.scope
}

public func check_export_sqlite_log(viewController: UIViewController) {
    if should_export_sqlite_log() {
        // the rust code has its own source of truth on this file name
        let db_fname = "data.db"
        let base = getDocumentsDirectory()
        let db_url = base.appendingPathComponent(db_fname)

        let desired_fname = "Scoria_Export_\(getFormattedDateTime()).sqlite"
        shareFileWithDifferentName(originalURL: db_url, desiredFilename: desired_fname, viewController: viewController)
    }
}

func getFormattedDateTime() -> String {
    let dateFormatter = DateFormatter()
    dateFormatter.dateFormat = "yyyy-MM-dd_HH-mm-ss"
    let formattedDateTime = dateFormatter.string(from: Date())
    return formattedDateTime
}

func check_import_sqlite_log(viewController: MyViewControllerProtocol) {
    if should_import_sqlite_log() {
        importFile(viewController: viewController)
    }
}

func check_export_track(viewController: UIViewController) {
    if should_export_track() {
        do {
            let fileManager = FileManager.default
            let dir = fileManager.temporaryDirectory
            let track_fname: String = "track_export"
            let directoryContents = try fileManager.contentsOfDirectory(
                at: dir,
                includingPropertiesForKeys: nil
            )
            for url in directoryContents {
                if url.lastPathComponent.starts(with: track_fname) {
                    let ext = url.pathExtension
                    let desiredFilename = "Scoria_Track_\(getFormattedDateTime()).\(ext)"
                    let new_url = dir.appendingPathComponent(desiredFilename)
                    try fileManager.moveItem(at: url, to: new_url)
                    shareFile(file: new_url, viewController: viewController, deleteAfterShare: true)
                    return
                }
            }
            print_and_log_error(s: "Failed to find a track file to export.")
        } catch {
            print_and_log_error(s: "Failed to either list directory contents or rename the track.")
        }
    }
}

public func handle_import(fileURL: URL) {
    print("Attempting to import data from \(fileURL)")
    let temporaryDirectory = FileManager.default.temporaryDirectory
    let temporaryURL = temporaryDirectory.appendingPathComponent("data_import.db")
    let fileManager = FileManager.default
    // Check if the destination file already exists
    if fileManager.fileExists(atPath: temporaryURL.path) {
        do {
            // Remove the existing file
            try fileManager.removeItem(at: temporaryURL)
        } catch {
            // Handle the error if unable to remove the file
            print_and_log_error(s: "Failed to remove existing temporary import file: \(error)")
            return
        }
    }
    if !fileURL.startAccessingSecurityScopedResource() {
        print_and_log_error(s: "Failed to access import file")
        return
    }
    do {
        try fileManager.copyItem(at: fileURL, to: temporaryURL)
    } catch {
        print_and_log_error(s: "Error copying import file: \(error)")
        return
    }
    fileURL.stopAccessingSecurityScopedResource()
    //print_and_log(s: "copied import file to \(temporaryURL)")
    import_from_sqlite_log(temporaryURL.path);

    // delete the temporary database file
    do {
        try fileManager.removeItem(at: temporaryURL)
    } catch {
        print_and_log_error(s: "Failed to remove temporary database file after import: \(error)")
    }
}
