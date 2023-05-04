import CoreLocation
import StemLib


class MyLocationManager: NSObject, CLLocationManagerDelegate, ObservableObject {
    let locationManager = CLLocationManager()
    
    let sqlURL: URL = getDocumentsDirectory().appendingPathComponent("data.db")
    
    override init() {
        super.init()
        
        locationManager.delegate = self
        updateConfig()
    }
    
    // If myLocationManager is stored as a global variable, it is lazily initialized.
    // `touch` does an access so that it becomes initialized.
    func touch() {
        print_and_log(s: "Initializing location manager")
    }
    
    func updateConfig() {
        setLocationEnabled()
        setSignificantChanges()
        setAccuracyMode()
        setDistanceFilter()
    }
    
    func requestPermissions() {
        locationManager.requestWhenInUseAuthorization()
        locationManager.requestAlwaysAuthorization()
        locationManager.pausesLocationUpdatesAutomatically = false
        // need to set this along with enabling in project background capabilities to get background updates:
        locationManager.allowsBackgroundLocationUpdates = true
    }

    // enable/disable location updating in general
    func setLocationEnabled() {
        let should_enable_location = get_location_enabled()
        //print_and_log(s: "setting location enabled to \(should_enable_location)")
        if should_enable_location {
            requestPermissions()
            locationManager.startUpdatingLocation()
        } else {
            locationManager.stopUpdatingLocation()
        }
    }

    // enable/disable the significant location changes service
    func setSignificantChanges() {
        let should_enable_slc = get_significant_changes()
        //print_and_log(s: "setting enable significant change mode to \(should_enable_slc)")
        if should_enable_slc {
            requestPermissions()
            locationManager.startMonitoringSignificantLocationChanges()
        } else {
            locationManager.stopMonitoringSignificantLocationChanges()
        }
    }
    
    // Set the minimum distance in meters the device must move horizontally before an update event is generated.
    func setDistanceFilter() {
        let dist_filt = get_distance_filter()
        //print_and_log(s: "setting dist filt to \(dist_filt)")
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
        //print_and_log(s: "setting accuracy to \(converted)")
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
        updateConfig()
        // Spawn an async task which will check if we should change auto modes after a minute
        // This is important in case the device suddenly stops, and we want to lower the
        // accuracy level to match the lack of movement.
        Task.detached {
            await self.updateConfigAfterOneMinute()
        }
        
    }
    
    // Waits one minute then calls checks if we should update the accuracy config
    func updateConfigAfterOneMinute() async {
        let startTime = DispatchTime.now()
        do {
            try await Task.sleep(nanoseconds: 1_000_000_000 * 60) // one minute
        } catch {
            print_and_log(s: "failed to sleep")
        }
        let endTime = DispatchTime.now()
        let elapsedTime = endTime.uptimeNanoseconds - startTime.uptimeNanoseconds
        let elapsedTimeInSeconds = Double(elapsedTime) / 1_000_000_000
        print_and_log(s: "checking location config after \(elapsedTimeInSeconds) seconds")
        updateConfig()
    }
}
