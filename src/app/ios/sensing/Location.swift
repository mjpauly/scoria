import CoreLocation
import StemLib


class MyLocationManager: NSObject, CLLocationManagerDelegate, ObservableObject {
    let locationManager = CLLocationManager()
        
    override init() {
        super.init()
        
        locationManager.delegate = self
        updateConfig()
    }
    
    // If myLocationManager is stored as a global variable, it is lazily initialized.
    // `touch` does an access so that it becomes initialized.
    func touch() {
        print("Initializing location manager")
    }
    
    // Determine if the location services are on so that background updates will arrive.
    // This boolean is used to update the state of lockscreen widgets.
    func isOn() -> Bool {
        let authorizedAlways = locationManager.authorizationStatus == .authorizedAlways
        let userEnabledLocation = get_location_enabled()
        return authorizedAlways && userEnabledLocation
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
        
        var story_available = false // default case
        var story = -1
        if let floor = newLocation.floor {
            story_available = true
            story = floor.level
        }
        var source_info_available = false; // default case
        var is_simulated_by_software = false;
        var is_produced_by_accessory = false;
        if let source_info = newLocation.sourceInformation {
            source_info_available = true;
            is_simulated_by_software = source_info.isSimulatedBySoftware
            is_produced_by_accessory = source_info.isProducedByAccessory
        }
        
        let data = OSLocationData.init(
            timestamp: Int64(round(newLocation.timestamp.timeIntervalSince1970)),
            
            latitude: newLocation.coordinate.latitude,
            longitude: newLocation.coordinate.longitude,
            horizontal_accuracy: newLocation.horizontalAccuracy,
            
            msl_altitude: newLocation.altitude,
            ellipsoid_altitude: newLocation.ellipsoidalAltitude,
            vertical_accuracy: newLocation.verticalAccuracy,
            
            story_available: story_available,
            story: Int64(story),
            
            speed: newLocation.speed,
            speed_accuracy: newLocation.speedAccuracy,
            course: newLocation.course,
            course_accuracy: newLocation.courseAccuracy,
            
            source_info_available: source_info_available,
            is_simulated_by_software: is_simulated_by_software,
            is_produced_by_accessory: is_produced_by_accessory
        )
        
        // Log the location in StemLib
        log_location(data)
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
        // let startTime = DispatchTime.now()
        do {
            try await Task.sleep(nanoseconds: 1_000_000_000 * 60) // one minute
        } catch {
            print_and_log_error(s: "failed to sleep")
        }
        // let endTime = DispatchTime.now()
        // let elapsedTime = endTime.uptimeNanoseconds - startTime.uptimeNanoseconds
        // let elapsedTimeInSeconds = Double(elapsedTime) / 1_000_000_000
        // print_and_log(s: "checking location config after \(elapsedTimeInSeconds) seconds")
        updateConfig()
    }
}
