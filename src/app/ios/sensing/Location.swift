import CoreLocation
import StemLib


class MyLocationManager: NSObject, CLLocationManagerDelegate, ObservableObject {
    let locationManager = CLLocationManager()
    
    let logURL: URL = getDocumentsDirectory().appendingPathComponent("gps_log.txt")
    let sqlURL: URL = getDocumentsDirectory().appendingPathComponent("data.db")
    
    override init() {
        super.init()
        
        locationManager.delegate = self
        locationManager.requestWhenInUseAuthorization()
        locationManager.requestAlwaysAuthorization()
        locationManager.pausesLocationUpdatesAutomatically = false  // default is false but doesn't hurt to set it
        // need to set this along with enabling in project background capabilities to get background updates:
        locationManager.allowsBackgroundLocationUpdates = true
        locationManager.startUpdatingLocation()
        setDistanceFilter()
    }
    
    // If myLocationManager is stored as a global variable, it is lazily initialized.
    // `touch` does an access so that it becomes initialized.
    func touch() {
        print("Initializing location manager")
    }
    
    // Set the minimum distance in meters the device must move horizontally before an update event is generated.
    func setDistanceFilter() {
        let dist_filt = get_distance_filter()
        locationManager.distanceFilter = CLLocationDistance(dist_filt)
    }
    
    // The locationManager() method of the CLLocationManagerDelegate protocol is called when the location manager receives new location data
    func locationManager(_ manager: CLLocationManager, didUpdateLocations locations: [CLLocation]) {
        // Perform operations on the updated location data
        guard let newLocation = locations.last else { return }
        //print(newLocation)
        appendLocationToFile(location: newLocation)
        
        // Log the location in StemLib
        log_location(
            newLocation.coordinate.latitude,
            newLocation.coordinate.longitude,
            newLocation.horizontalAccuracy,
            newLocation.speed,
            newLocation.course,
            Int(round(newLocation.timestamp.timeIntervalSince1970))
        )
        
    }
    
    func appendLocationToFile(location: CLLocation) {
        let dataString = "\(location)\n"
        appendToFile(file: logURL.path(), dataString: dataString)
    }
}
