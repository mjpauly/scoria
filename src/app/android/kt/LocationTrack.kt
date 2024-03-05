/* Location object altitude fields
 *
 * getAltitude()                    valid if hasAltitude()
 * getVerticalAccuracyMeters()      valid if hasVerticalAccuracy(l
 *
 * getMslAltitudeMeters()           valid if hasMslAltitude()
 * getMslAltitudeAccuracyMeters()   valid if hasMslAltitudeAccuracy()
 */

package info.scoria

import java.lang.Runnable
import java.util.concurrent.Executor
import android.Manifest
import android.app.Activity;
import android.app.Service
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.location.Location
import android.location.LocationListener
import android.location.LocationManager
import android.os.Bundle
import android.os.IBinder
import android.provider.Settings
import android.util.Log

class LocationTrack(private val context: Context) : Service(), LocationListener {

    private val locationManager: LocationManager

    private val MIN_DISTANCE_CHANGE_FOR_UPDATES: Float = 10.0f;
    private val MIN_TIME_BW_UPDATES: Long = 1000;

    init {
        locationManager = context.getSystemService(LOCATION_SERVICE) as LocationManager
    }

    public fun startListener() {
        System.out.println("starting listener")
        // get GPS status
        val gpsEnabled = locationManager.isProviderEnabled(LocationManager.GPS_PROVIDER);

        // get network provider status
        val networkEnabled = locationManager.isProviderEnabled(LocationManager.NETWORK_PROVIDER);

        if (!gpsEnabled && !networkEnabled) {
            Log.w("scoria.info", "No Service Provider is available")
            return
        }

        // TODO: pick provider based on desired accuracy level
        // getDistanceFilter()
        // getLocationAccuracyMode()
        // getSignificantChanges()

        // if GPS Enabled get lat/long using GPS Services
        if (gpsEnabled) {
            System.out.println("starting GPS")
            locationManager.requestLocationUpdates(
                LocationManager.GPS_PROVIDER,
                MIN_TIME_BW_UPDATES,
                MIN_DISTANCE_CHANGE_FOR_UPDATES,
                this
            )
        }

        // More refined request:
        // var request: LocationRequest = ...
        // requestLocationUpdates(
        //     LocationManager.FUSED_PROVIDER,
        //     request,
        //     DirectExecutor(),
        //     this
        // )
    }

    public fun stopListener() {
        locationManager.removeUpdates(this)
    }

    public override fun onBind(intent: Intent): IBinder? {
        return null;
    }

    public override fun onLocationChanged(loc: Location) {
        System.out.println("New location: ${loc.getLongitude()}, ${loc.getLatitude()}")


        val osloc = OSLocationData()

        // these fields are always valid
        osloc.timestamp = loc.getTime() / 1000
        osloc.latitude = loc.getLatitude()
        osloc.longitude = loc.getLongitude()
        osloc.horizontal_accuracy = loc.getAccuracy().toDouble()

        // only log altitude when both ellipsoid and msl data is available
        // TODO: handle case where we might have one but not the other?
        if (loc.hasAltitude()
            && loc.hasMslAltitude()
            && loc.hasVerticalAccuracy()
        ) {
            osloc.msl_altitude = loc.getMslAltitudeMeters()
            osloc.ellipsoid_altitude = loc.getAltitude()
            // assuming that vertical accuracy is ~the same as msl accuracy
            osloc.vertical_accuracy = loc.getVerticalAccuracyMeters().toDouble()
        } else {
            osloc.msl_altitude = -1.0
            osloc.ellipsoid_altitude = -1.0
            osloc.vertical_accuracy = -1.0 // -1.0 indicates no altitude data
        }

        osloc.story_available = false // no equivalent on Android
        osloc.story = -1

        osloc.speed = if (loc.hasSpeed())
            loc.getSpeed().toDouble() else -1.0
        osloc.speed_accuracy = if (loc.hasSpeedAccuracy())
            loc.getSpeedAccuracyMetersPerSecond().toDouble() else -1.0
        osloc.course = if (loc.hasBearing())
            loc.getBearing().toDouble() else -1.0
        osloc.course_accuracy = if (loc.hasBearingAccuracy())
            loc.getBearingAccuracyDegrees().toDouble() else -1.0

        osloc.source_info_available = true
        osloc.is_simulated_by_software = loc.isMock()
        osloc.is_produced_by_accessory = false // no equivalent on Android

        Stem.logLocation(osloc)
    }

    public override fun onProviderEnabled(provider: String) {
        System.out.println("Provider enabled: ${provider}")
    }

    public override fun onProviderDisabled(provider: String) {
        System.out.println("Provider disabled: ${provider}")
    }
}

class DirectExecutor : Executor {
     override fun execute(r: Runnable) {
        r.run();
     }
}
