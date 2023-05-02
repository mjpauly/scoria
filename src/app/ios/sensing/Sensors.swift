import StemLib

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

public func update_sensor_config() {
    // load config from backend since we got poked by the frontend
    myLocationManager.setLocationEnabled()
    myLocationManager.setSignificantChanges()
    myLocationManager.setAccuracyMode()
    myLocationManager.setDistanceFilter()
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
