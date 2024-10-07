import CoreMotion
import Foundation
import StemLib

let UPDATE_RATE_HZ = 100.0
let ONE_G = 9.81

class MyMotionManager: NSObject {
    let motion = CMMotionManager()
    var timer: Timer? = nil

    func startIMU() {
        // Make sure the IMU hardware is available. 
        if !(self.motion.isAccelerometerAvailable && self.motion.isGyroAvailable) { return }

        self.motion.accelerometerUpdateInterval = 1.0 / UPDATE_RATE_HZ // T
        self.motion.gyroUpdateInterval = 1.0 / UPDATE_RATE_HZ // T
        self.motion.startAccelerometerUpdates()
        self.motion.startGyroUpdates()

        // Configure a timer to fetch the data.
        self.timer = Timer(fire: Date(), interval: (1.0 / UPDATE_RATE_HZ), 
                  repeats: true, block: { (timer) in
            guard let accelerometerData = self.motion.accelerometerData else { return }
            guard let gyroData = self.motion.gyroData else { return }

            let a = ThreeAxisData.init(
                x: accelerometerData.acceleration.x * ONE_G,
                y: accelerometerData.acceleration.y * ONE_G,
                z: accelerometerData.acceleration.z * ONE_G
            )

            let g = ThreeAxisData.init(
                x: gyroData.rotationRate.x,
                y: gyroData.rotationRate.y,
                z: gyroData.rotationRate.z
            )

            imu_data(a, g)
        })

        // Add the timer to the current run loop.
        RunLoop.current.add(self.timer!, forMode: RunLoop.Mode.default)
    }

    func stopIMU() {
        self.timer?.invalidate()
        self.timer = nil
        self.motion.stopAccelerometerUpdates()
        self.motion.stopGyroUpdates()
    }

    let queue = OperationQueue()

    func startDeviceMotionUpdates() {
        if !self.motion.isDeviceMotionAvailable { return }

        self.motion.deviceMotionUpdateInterval = 1.0 / UPDATE_RATE_HZ
        self.motion.startDeviceMotionUpdates(using: .xTrueNorthZVertical, 
                  to: self.queue, withHandler: { (data, error) in
            // Make sure the data is valid before accessing it.
            if let validData = data {
                // Get the attitude relative to the magnetic north reference frame. 
                let roll = validData.attitude.roll
                let pitch = validData.attitude.pitch
                let yaw = validData.attitude.yaw
  
  
                // Use the motion data in your app.
            }
        })

    }

    func stopDeviceMotionUpdates() {
        self.motion.stopDeviceMotionUpdates()
    }

}


/*
    public func startAccelerometer() {
        // Make sure the accelerometer hardware is available. 
        if !self.motion.isAccelerometerAvailable { return }
        self.motion.accelerometerUpdateInterval = 1.0 / UPDATE_RATE_HZ // T
        self.motion.startAccelerometerUpdates(to: .main) { accelerometerData, error in
            guard let data = accelerometerData else { return }

            // Data in g's (multiply by 9.81 for m/s^2)
            let x = data.acceleration.x * ONE_G
            let y = data.acceleration.y * ONE_G
            let z = data.acceleration.z * ONE_G

            // accelerometer_data(x, y, z)

            // print("[\(x), \(y), \(z)],")
        }
    }
    
    public func stopAccelerometer() {
        self.motion.stopAccelerometerUpdates()
    }

    public func startGyro() {
        if !self.motion.isGyroAvailable { return }
        self.motion.gyroUpdateInterval = 1.0 / UPDATE_RATE_HZ // T
        self.motion.startGyroUpdates(to: .main) { gyroData, error in
            guard let data = gyroData else { return }

            // Data in radians/s
            let x = data.rotationRate.x
            let y = data.rotationRate.y
            let z = data.rotationRate.z

            // gyro_data(x, y, z)

            // print("[\(x), \(y), \(z)],")
        }

    }

    public func stopGyro() {
        self.motion.stopGyroUpdates()
    }
    */
