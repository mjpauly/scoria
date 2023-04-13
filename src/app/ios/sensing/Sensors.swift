import StemLib

var myLocationManager = MyLocationManager()
public var server_config = ServerConfig();  // struct defined in StemLib.h

public func startup() {
    // Set the app directories known to the core library and get the
    // backend server port and secret key
    server_config = set_app_dirs(getDocumentsDirectory().path(),
                 getLibraryDirectory().path(),
                 getTemporaryDirectoryPath(),
                 getBundlePath())
    myLocationManager.touch()  // initialize the lazy global var
}

public func update_sensor_config() {
    // load config from backend since we got poked by the frontend
    myLocationManager.setLocationEnabled()
    myLocationManager.setSignificantChanges()
    myLocationManager.setAccuracyMode()
    myLocationManager.setDistanceFilter()
}
