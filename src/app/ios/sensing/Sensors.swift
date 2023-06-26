import StemLib
import UIKit

// Shared type understanding by the Sensing and Top frameworks of what the ViewController can do.
public protocol MyViewControllerProtocol: UIViewController, UIDocumentPickerDelegate {}

var myLocationManager = MyLocationManager()
public var server_port: UInt16 = 0;
public var server_scope: UInt64 = 0;
public var logfile = getDocumentsDirectory().appendingPathComponent("swiftlog.txt").path()

public func startup() {
    // Set the app directories known to the core library and get the
    // backend server port and secret key
    set_app_dirs(getDocumentsDirectory().path(),
                 getLibraryDirectory().path(),
                 getTemporaryDirectoryPath(),
                 getBundlePath())
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
}

// check if we should prompt for when-in-use authorization
func check_request_when_in_use_authorization() {
    if should_request_when_in_use_authorization() {
        myLocationManager.locationManager.requestWhenInUseAuthorization()
    }
}

public func print_and_log(s: String) {
    print(s)
    appendToFile(file: logfile, dataString: "\(s)\n")
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
        
        let dateFormatter = DateFormatter()
        dateFormatter.dateFormat = "yyyy-MM-dd_HH-mm-ss"
        let formattedDateTime = dateFormatter.string(from: Date())
        let desired_fname = "Epsilon_Export_\(formattedDateTime).sqlite"
        
        shareFileWithDifferentName(originalURL: db_url, desiredFilename: desired_fname, viewController: viewController)
    }
}

func check_import_sqlite_log(viewController: MyViewControllerProtocol) {
    if should_import_sqlite_log() {
        importFile(viewController: viewController)
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
            print_and_log(s: "Failed to remove existing temporary import file: \(error)")
            return
        }
    }
    if !fileURL.startAccessingSecurityScopedResource() {
        print_and_log(s: "Failed to access import file")
        return
    }
    do {
        try fileManager.copyItem(at: fileURL, to: temporaryURL)
    } catch {
        print_and_log(s: "Error copying import file: \(error)")
        return
    }
    fileURL.stopAccessingSecurityScopedResource()
    print_and_log(s: "copied import file to \(temporaryURL)")
    import_from_sqlite_log(temporaryURL.path);

    // delete the temporary database file
    do {
        try fileManager.removeItem(at: temporaryURL)
    } catch {
        print_and_log(s: "Failed to remove temporary database file after import: \(error)")
    }
}
