import CoreMotion
import Foundation
import StemLib


class MyMotionManager: NSObject {
    let motion = CMMotionManager()
    var timer: Timer? = nil

    // Trigger initialization of a lazily-initialized instance
    func touch() {
        print("Initializing location manager")
    }

    func startAccelerometers() {
        // Make sure the accelerometer hardware is available. 
        if self.motion.isAccelerometerAvailable {
             self.motion.accelerometerUpdateInterval = 1.0 / 50.0  // 50 Hz
             self.motion.startAccelerometerUpdates()


             // Configure a timer to fetch the data.
             self.timer = Timer(fire: Date(), interval: (1.0/50.0), 
                       repeats: true, block: { (timer) in
                  // Get the accelerometer data.
                  if let data = self.motion.accelerometerData {
                       let x = data.acceleration.x
                       let y = data.acceleration.y
                       let z = data.acceleration.z

                       print("x: \(x), y: \(y), z: \(z)")

                       // Use the accelerometer data
                  }
             })


            // Add the timer to the current run loop.
            RunLoop.current.add(self.timer!, forMode: RunLoop.Mode.default)
        }
    }

}
