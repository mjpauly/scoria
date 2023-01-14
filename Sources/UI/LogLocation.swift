//
//  LogLocation.swift
//  firstproj
//
//  Created by Matthew Pauly on 12/7/22.
//

import Foundation
import CoreLocation


class MyLocationManager: NSObject, CLLocationManagerDelegate, ObservableObject {
    let locationManager = CLLocationManager()
    
    @Published var currentLocation: CLLocation = CLLocation()
    @Published var updatesThisHour: Int = 0
    let logURL: URL = getDocumentsDirectory().appendingPathComponent("gps_log.txt")
    
    override init() {
        super.init()
        
        locationManager.delegate = self
        locationManager.requestWhenInUseAuthorization()
        locationManager.requestAlwaysAuthorization()
        locationManager.pausesLocationUpdatesAutomatically = false  // default is false but doesn't hurt to set it
        // need to set this along with enabling in project background capabilities to get background updates:
        locationManager.allowsBackgroundLocationUpdates = true
        locationManager.startUpdatingLocation()
    }
    
    // The locationManager() method of the CLLocationManagerDelegate protocol is called when the location manager receives new location data
    func locationManager(_ manager: CLLocationManager, didUpdateLocations locations: [CLLocation]) {
        // Perform operations on the updated location data
        guard let newLocation = locations.last else { return }
        //print(newLocation)
        calcUpdatesThisHour()  // must come before updating currentLocation
        appendLocationToFile(location: newLocation)
        currentLocation = newLocation
    }
    
    func calcUpdatesThisHour() {
        let prevHour = Calendar.current.component(.hour, from: self.currentLocation.timestamp)
        let currHour = Calendar.current.component(.hour, from: Date())
        if prevHour != currHour {
            updatesThisHour = 0
        }
        updatesThisHour += 1
    }
    
    func appendLocationToFile(location: CLLocation) {
        let dataString = "\(location)\n"
        appendToFile(file: logURL.path(), dataString: dataString)
        //writeToNewFile(file: logURL.path(), dataString: dataString)  // for testing, easier to see changes
    }
    
    
}

// FILE stuff

// appendToFile tries to append data to a file, and if the file doesn't exist it creates it
func appendToFile(file: String, dataString: String) {
    /* try to append to file if file exists, otherwise create new file with data */
    let data = dataString.data(using: .utf8)!  // Encode String as bytes for file writing
    let fileManager = FileManager.default
    if fileManager.fileExists(atPath: file) {  // Check if the file exists
        let fileHandle = FileHandle(forWritingAtPath: file)  // Open the file for writing
        fileHandle?.seekToEndOfFile()  // Move to the end of the file
        fileHandle?.write(data)  // Append the data to the file
        fileHandle?.closeFile()  // Close the file
    } else {
        writeToNewFile(file: file, dataString: dataString)
    }
}

// writeToNewFile writes data to a file, creating it if it doesn't exist, or overwriting it if it does
func writeToNewFile(file: String, dataString: String) {
    /* Write the data to a new file, overwriting what was there before */
    do {
        try dataString.write(toFile: file, atomically: true, encoding: .utf8)
    } catch {
        print("Error writing to file: \(error)")
    }
}
