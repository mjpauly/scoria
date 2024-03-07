/* Location object altitude fields
 *
 * getAltitude()                    valid if hasAltitude()
 * getVerticalAccuracyMeters()      valid if hasVerticalAccuracy(l
 *
 * getMslAltitudeMeters()           valid if hasMslAltitude()
 * getMslAltitudeAccuracyMeters()   valid if hasMslAltitudeAccuracy()
 */

package info.scoria

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.Service
import android.content.Context
import android.content.Intent
import android.graphics.Color
import android.location.Location
import android.location.LocationListener
import android.location.LocationManager
import android.os.Build
import android.os.IBinder
import android.util.Log
import androidx.annotation.RequiresApi
import androidx.core.app.NotificationCompat
import java.lang.Runnable
import java.util.concurrent.Executor

class LocationService() : Service(), LocationListener {

    private val TAG = "LocationService"

    private val MIN_TIME_BW_UPDATES: Long = 1000;

    override fun onCreate() {
        Log.i(TAG, "starting")
        // Android may start this service without the MainActivity, so we need
        // to ensure stem is initialized
        Stem.handleStartup(
            getFilesDir().getAbsolutePath(),
            getCacheDir().getAbsolutePath()
        )
        createNotificationChanel()
        if (Build.VERSION.SDK_INT > Build.VERSION_CODES.O) {
            createNotificationChanel()
        } else {
            startForeground(1, Notification())
        }
    }

    @RequiresApi(Build.VERSION_CODES.O)
    private fun createNotificationChanel() {
        val NOTIFICATION_CHANNEL_ID = "info.scoria"
        val channelName = "Scoria Location"
        val chan = NotificationChannel(
            NOTIFICATION_CHANNEL_ID,
            channelName,
            NotificationManager.IMPORTANCE_NONE
        )
        chan.lightColor = Color.BLUE
        chan.lockscreenVisibility = Notification.VISIBILITY_PRIVATE
        val manager =
            (getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager)
        manager.createNotificationChannel(chan)
        val notification: Notification =
            NotificationCompat.Builder(this, NOTIFICATION_CHANNEL_ID)
                .setOngoing(true)
                .setContentTitle("Scoria is running")
                .setPriority(NotificationManager.IMPORTANCE_MIN)
                .setCategory(Notification.CATEGORY_SERVICE)
                .build()
        startForeground(2, notification)
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        super.onStartCommand(intent, flags, startId)
        requestLocationUpdates()
        return START_STICKY
    }

    override fun onDestroy() {
        val locationManager = getSystemService(LOCATION_SERVICE) as LocationManager
        locationManager.removeUpdates(this)
        super.onDestroy()
        Log.i(TAG, "onDestroy")
    }

    public override fun onBind(intent: Intent): IBinder? {
        return null;
    }

    private fun requestLocationUpdates() {
        // get GPS and network location provicer status
        val locationManager = getSystemService(LOCATION_SERVICE) as LocationManager
        val gpsEnabled = locationManager.isProviderEnabled(LocationManager.GPS_PROVIDER);
        // val networkEnabled = locationManager.isProviderEnabled(LocationManager.NETWORK_PROVIDER);

        // if (!gpsEnabled && !networkEnabled) {
        if (!gpsEnabled) {
            Log.w(TAG, "No Service Provider is available")
            stopSelf()
            return
        }

        // TODO: pick provider based on desired accuracy level
        val minDistanceChange = Stem.getDistanceFilter();
        // getLocationAccuracyMode()
        // getSignificantChanges()

        Log.i(TAG, "starting GPS")
        locationManager.requestLocationUpdates(
            LocationManager.GPS_PROVIDER,
            MIN_TIME_BW_UPDATES,
            minDistanceChange,
            this
        )

        // More refined request:
        // var request: LocationRequest = ...
        // requestLocationUpdates(
        //     LocationManager.FUSED_PROVIDER,
        //     request,
        //     DirectExecutor(),
        //     this
        // )
    }

    public override fun onLocationChanged(loc: Location) {
        Log.d(TAG, "New location: ${loc.getLongitude()}, ${loc.getLatitude()}")

        val osloc = OSLocationData()

        // these fields are always valid
        osloc.timestamp = loc.getTime() / 1000
        osloc.latitude = loc.getLatitude()
        osloc.longitude = loc.getLongitude()
        osloc.horizontal_accuracy = loc.getAccuracy().toDouble()

        // only log altitude when both ellipsoid and msl data is available
        // TODO: handle case where we might have one but not the other?
        if (loc.hasAltitude()
            && loc.hasVerticalAccuracy()
            && loc.hasMslAltitude()
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
        Log.i(TAG, "Provider enabled: ${provider}")
    }

    public override fun onProviderDisabled(provider: String) {
        Log.i(TAG, "Provider disabled: ${provider}")
    }
}

class DirectExecutor : Executor {
     override fun execute(r: Runnable) {
        r.run();
     }
}
