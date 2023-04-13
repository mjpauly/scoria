import CoreLocation
import StemLib


class MyLocationManager: NSObject, CLLocationManagerDelegate, ObservableObject {
    let locationManager = CLLocationManager()
    
    let sqlURL: URL = getDocumentsDirectory().appendingPathComponent("data.db")
    
    override init() {
        super.init()
        
        locationManager.delegate = self
        setLocationEnabled()
        setSignificantChanges()
        setAccuracyMode()
        setDistanceFilter()
    }
    
    // If myLocationManager is stored as a global variable, it is lazily initialized.
    // `touch` does an access so that it becomes initialized.
    func touch() {
        print("Initializing location manager")
    }

    // enable/disable location updating in general
    func setLocationEnabled() {
        let should_enable_location = get_location_enabled()
        if should_enable_location {
            locationManager.requestWhenInUseAuthorization()
            locationManager.requestAlwaysAuthorization()
            locationManager.pausesLocationUpdatesAutomatically = false
            // need to set this along with enabling in project background capabilities to get background updates:
            locationManager.allowsBackgroundLocationUpdates = true
            locationManager.startUpdatingLocation()
        } else {
            locationManager.stopUpdatingLocation()
        }
    }

    // enable/disable the significant location changes service
    func setSignificantChanges() {
        let should_enable_slc = get_significant_changes()
        if should_enable_slc {
            locationManager.startMonitoringSignificantLocationChanges()
        } else {
            locationManager.stopMonitoringSignificantLocationChanges()
        }
    }
    
    // Set the minimum distance in meters the device must move horizontally before an update event is generated.
    func setDistanceFilter() {
        let dist_filt = get_distance_filter()
        locationManager.distanceFilter = CLLocationDistance(dist_filt)
    }

    func setAccuracyMode() {
        let accuracy_mode = get_location_accuracy_mode()
        //let converted = kCLLocationAccuracyBest
        var converted: CLLocationAccuracy;
        switch accuracy_mode {
            case Best: converted = kCLLocationAccuracyBest
            case TenMeters: converted = kCLLocationAccuracyNearestTenMeters
            case HundredMeters: converted = kCLLocationAccuracyHundredMeters
            case Kilometer: converted = kCLLocationAccuracyKilometer
            case ThreeKilometers: converted = kCLLocationAccuracyThreeKilometers
        default:
            converted = kCLLocationAccuracyBest
        }
        locationManager.desiredAccuracy = converted
    }
    
    // The locationManager() method of the CLLocationManagerDelegate protocol is called when the location manager receives new location data
    func locationManager(_ manager: CLLocationManager, didUpdateLocations locations: [CLLocation]) {
        // Perform operations on the updated location data
        guard let newLocation = locations.last else { return }
        
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
}
