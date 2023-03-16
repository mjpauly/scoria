import StemLib

var myLocationManager = MyLocationManager()

public func startup() {
    // Set the app directories known to the core library
    set_app_dirs(getDocumentsDirectory().path(),
                 getLibraryDirectory().path(),
                 getTemporaryDirectoryPath(),
                 getBundlePath())
    myLocationManager.touch()  // initialize the lazy global var
}

public func update_sensor_config() {
    // load config from backend since we got poked by the frontend
    myLocationManager.setDistanceFilter()
}
