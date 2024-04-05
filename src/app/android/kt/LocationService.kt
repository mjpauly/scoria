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
import android.location.LocationManager
import android.os.Binder
import android.os.Build
import android.os.Handler
import android.os.IBinder
import android.os.Looper
import android.util.Log
import androidx.annotation.RequiresApi
import androidx.core.app.NotificationCompat
import androidx.core.location.LocationCompat
import androidx.core.location.LocationListenerCompat
import androidx.core.location.LocationManagerCompat
import androidx.core.location.LocationRequestCompat
import java.lang.Runnable
import java.util.Timer
import java.util.TimerTask
import java.util.concurrent.Executor

class LocationService() : Service(), LocationListenerCompat {

    private val TAG = "LocationService"

    private val UPDATE_INTERVAL_MS: Long = 1000;
    private val UPDATE_CONFIG_DELAY_MS: Long = 1000 * 60 * 5; // 5 minutes

    // Binder channel for MainActivity to directly call methods on the service
    private val binder = LocalBinder()
    
    // Callback to MainActivity to update the location config
    private var callback: UpdateConfigCallback? = null

    // Timer for scheduling delayed location config updates
    private var timer: Timer? = null

    override fun onCreate() {
        // Log.i(TAG, "starting on thread: ${java.lang.Thread.currentThread().getName()}")
        // Android may start this service without the MainActivity, so we need
        // to ensure stem is initialized
        Stem.handleStartup(
            getFilesDir().getAbsolutePath(),
            getCacheDir().getAbsolutePath(),
            getVersionName(this)
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
        Log.i(TAG, "onStartCommand")
        requestLocationUpdates()
        return START_STICKY
    }

    override fun onDestroy() {
        val locationManager = getSystemService(LOCATION_SERVICE) as LocationManager
        LocationManagerCompat.removeUpdates(locationManager, this)
        super.onDestroy()
        Log.i(TAG, "onDestroy")
    }

    private fun requestLocationUpdates() {
        val locationManager = getSystemService(LOCATION_SERVICE) as LocationManager
        val gpsEnabled = locationManager.isProviderEnabled(LocationManager.GPS_PROVIDER);
        val networkEnabled = locationManager.isProviderEnabled(LocationManager.NETWORK_PROVIDER);
        val fusedEnabled = locationManager.isProviderEnabled(LocationManager.FUSED_PROVIDER);
        Log.i(TAG, "providers - gps: ${gpsEnabled}, network: ${networkEnabled}, fused: ${fusedEnabled}")
        if (!LocationManagerCompat.isLocationEnabled(locationManager)
            || !Stem.getLocationEnabled()
        ) {
            Log.w(TAG, "Location is not enabled")
            stopSelf()
            return
        }

        Log.i(TAG, "starting location updates")
        
        // Our "significant changes" mode will passively listen for location
        // updates triggered by the rest of the system

        // TODO: since every listener/provider pair can be registered, do we
        // only ever want to use the FUSED provider?
        // val provider =
            // if (Stem.getSignificantChanges())
                // LocationManager.PASSIVE_PROVIDER
            // else
                // LocationManager.FUSED_PROVIDER
        val provider = LocationManager.FUSED_PROVIDER
        val interval =
            if (Stem.getSignificantChanges())
                LocationRequestCompat.PASSIVE_INTERVAL
            else
                UPDATE_INTERVAL_MS
        val minDistanceChange = Stem.getDistanceFilter()
        val accuracyMode = Stem.getLocationAccuracyMode()
        val quality = when {
            accuracyMode < 100.0 -> LocationRequestCompat.QUALITY_HIGH_ACCURACY
            accuracyMode < 1000.0 -> LocationRequestCompat.QUALITY_BALANCED_POWER_ACCURACY
            else -> LocationRequestCompat.QUALITY_LOW_POWER
        }
        val request = LocationRequestCompat.Builder(interval)
            .setMinUpdateDistanceMeters(minDistanceChange)
            .setQuality(quality)
            .build()

        LocationManagerCompat.requestLocationUpdates(
            locationManager,
            provider,
            request,
            this,
            // need to log to Stem on the main thread, or we will get SELinux denials
            Looper.getMainLooper(),
        )
    }

    public override fun onLocationChanged(locations: MutableList<Location>) {
        for (loc in locations) {
            logLocation(loc)
        }
    }

    public override fun onLocationChanged(loc: Location) {
        logLocation(loc)
    }

    /* On API levels 29 & 30, requesting location updates always results in
     * delivery of the last location, creating a continuous feedback loop of
     * repeated location delivery. To avoid this, we check if the new location
     * satisfies the distance filter requirement before persisting it and
     * updating the location config.
     */
    private var lastLoc: Location? = null // last persisted location

    private fun logLocation(loc: Location) {
        Log.d(TAG, "New location received")

        lastLoc?.let {
            if (loc.distanceTo(it) < Stem.getDistanceFilter()) {
                // new data doesn't satisfy distance filter
                startTimer()
                return
            }
        }
        lastLoc = loc

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
            // use LocationCompat to handle older API levels
            && LocationCompat.hasMslAltitude(loc)  // needed under API level 34
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
        osloc.is_simulated_by_software = LocationCompat.isMock(loc) // needed under 31
        osloc.is_produced_by_accessory = false // no equivalent on Android

        Stem.logLocation(osloc)

        callback?.updateConfigCallback()
        startTimer()
    }

    // Updates the location config after a delay, in case the device stopped
    // moving and location updates have stopped abruptly.
    private fun startTimer() {
        timer?.let { it.cancel() }
        timer = Timer()
        val timerTask = object : TimerTask() {
            override fun run() {
                Log.i(TAG, "Timer run")
                callback?.updateConfigCallback()
            }
        }
        timer!!.schedule(timerTask, UPDATE_CONFIG_DELAY_MS)
    }

    inner class LocalBinder : Binder() {
        fun getService(): LocationService = this@LocationService
    }

    public override fun onBind(intent: Intent): IBinder? {
        return binder
    }

    public fun registerCallback(callback: UpdateConfigCallback) {
        this.callback = callback
    }

    public override fun onProviderEnabled(provider: String) {
        Log.i(TAG, "Provider enabled: ${provider}")
    }

    public override fun onProviderDisabled(provider: String) {
        Log.i(TAG, "Provider disabled: ${provider}")
    }
}
