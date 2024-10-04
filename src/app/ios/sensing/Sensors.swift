import StemLib
import UIKit

// Shared type understanding by the Sensing and Top code regarding what the ViewController can do.
public protocol MyViewControllerProtocol: UIViewController, UIDocumentPickerDelegate {}

var myLocationManager = MyLocationManager()
var myMotionManager: MyMotionManager? = nil
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
    myMotionManager = MyMotionManager()
    print("starting imu");
    myMotionManager?.startIMU()
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
    check_import(viewController: viewController)
    check_export_track(viewController: viewController)
    check_request_notifications()
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

public func handle_url_scheme(URLContexts: Set<UIOpenURLContext>) {
    if let urlContext = URLContexts.first {
        url_scheme(urlContext.url.absoluteString)
    }
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

func check_import(viewController: MyViewControllerProtocol) {
    if should_import_sqlite_log() {
        importFile(
            viewController: viewController, presentationSource: .database
        )
    } else if should_import_places_geojson() {
        importFile(
            viewController: viewController, presentationSource: .placesGeojson
        )
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

// Database imports have to copy the import database to a new location so that
// the database can be migrated.
public func handle_database_import(fileURL: URL) {
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

public func handle_places_geojson_import(fileURL: URL) {
    if !fileURL.startAccessingSecurityScopedResource() {
        print_and_log_error(s: "Failed to access import file")
        return
    }
    import_places_geojson(fileURL.path);
    fileURL.stopAccessingSecurityScopedResource()
}

// ask for user permission to show notifications when the app is quit
func check_request_notifications() {
    if should_notify_on_stop() {
        UNUserNotificationCenter.current().requestAuthorization(
            options: [.alert, .sound]
        ) { granted, error in
            if let error = error {
                print("Error requesting notification permissions: \(error)")
            }
        }
    }
}

public func scheduleStopNotification() {
    // only notify if user has enabled locations and granted always
    // authorization
    if should_notify_on_stop() && myLocationManager.isOn() {
        // Create a notification content object
        let content = UNMutableNotificationContent()
        content.title = "Scoria Stopped"
        content.body = "Keep Scoria open in the background to log locations."
        content.sound = UNNotificationSound.default

        // Create a notification trigger
        let trigger =
            UNTimeIntervalNotificationTrigger(timeInterval: 1, repeats: false)

        // Create a notification request
        let request = UNNotificationRequest(
            identifier: "scoria",
            content: content,
            trigger: trigger
        )

        // Add the notification request to the notification center
        UNUserNotificationCenter.current().add(request) { error in
            if let error = error {
                print_and_log_error(s: "Error scheduling notification: \(error)")
            }
        }
    }
}
