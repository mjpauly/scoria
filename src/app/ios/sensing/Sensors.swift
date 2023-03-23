import StemLib

var myLocationManager = MyLocationManager()
public var server_port: UInt16 = 0

public func startup() {
    // Set the app directories known to the core library
    server_port = set_app_dirs(getDocumentsDirectory().path(),
                 getLibraryDirectory().path(),
                 getTemporaryDirectoryPath(),
                 getBundlePath())
    myLocationManager.touch()  // initialize the lazy global var
}

public func update_sensor_config() {
    // load config from backend since we got poked by the frontend
    myLocationManager.setDistanceFilter()
}
