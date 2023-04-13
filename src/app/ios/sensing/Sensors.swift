import StemLib

var myLocationManager = MyLocationManager()
public var server_port: UInt16 = 0;
public var server_scope: UInt64 = 0;

public func startup() {
    // Set the app directories known to the core library and get the
    // backend server port and secret key
    let server_config = set_app_dirs(getDocumentsDirectory().path(),
                 getLibraryDirectory().path(),
                 getTemporaryDirectoryPath(),
                 getBundlePath())
    server_port = server_config.port
    server_scope = server_config.scope
    myLocationManager.touch()  // initialize the lazy global var
}

public func update_sensor_config() {
    // load config from backend since we got poked by the frontend
    myLocationManager.setLocationEnabled()
    myLocationManager.setSignificantChanges()
    myLocationManager.setAccuracyMode()
    myLocationManager.setDistanceFilter()
}
