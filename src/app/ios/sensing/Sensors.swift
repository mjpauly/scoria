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
