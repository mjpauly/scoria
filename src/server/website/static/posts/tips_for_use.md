# Tips for Using Scoria

*Sep 3, 2024*

## <a id="continuous-logging" href="#continuous-logging">Ensuring Continuous Logging</a>

You may have noticed times when Scoria stopped logging your movement without
you changing anything. Opening the app expecting to have hours or days of new
activity to analyze, only to find your movement wasn't logged, can be
frustrating.

Scoria gets location updates from your device's OS, and it must be running in
the background to receive locations. There are a couple reasons why Scoria might
stop running.

- Scoria hasn't been opened in the past day. Location services take power to
  run, and the OS assumes that you don't want to be losing battery life to apps
  that you're not actively using. The same can be said for privacy loss, since
  most apps are not as [privacy-forward](/privacy) as Scoria, and this policy
  helps protect your privacy in the average case. On iOS, Scoria may be shut
  down in as little as 24 hours since it was last opened.
- Your device's battery level is low, or it's in low-power mode. The OS may
  shut down Scoria to save battery life.
- Scoria was updated. After an update, Scoria might not start running again
  until you reopen it.
- Scoria was shut down in the app switcher.

In all of these cases, it's necessary to reopen Scoria for it to start logging
locations again.

## <a id="power-use" href="#power-use">Factors of Power Use</a>

Sometimes you may find that Scoria drains your battery more than expected.

When using Automatic Mode, Scoria's power consumption depends on how much you
move around. Whenever you're stationary, Scoria saves power by switching to a
lower-accuracy location mode behind the scenes. When it detects that you're
moving, it changes to a higher-accuracy mode, better capturing the details of
your movement. If you don't usually spend most of the day in motion, a full day
of movement can consume more battery than you might expect. When traveling a
lot, we recommend using an external battery to ensure your device stays charged
while still logging accurate location data.

Alternatively, you can switch to Reduced Mode if you don't mind a decrease in
data accuracy. In Reduced Mode, power consumption doesn't change according to
your movement as much.

Conditions that reduce data accuracy, as discussed in the next section, can also
increase power use, as your device has to work harder to produce data with the
same accuracy.

## <a id="data-accuracy" href="#data-accuracy">Factors of Data Accuracy</a>

The data Scoria collects can often be inaccurate, and there's many reasons why
your device can produce low-accuracy location data. For example:

- There might be obstructions that block or reflect signals from
  [GPS/GNSS](https://en.wikipedia.org/wiki/Satellite_navigation) satellites,
  like tall buildings in urban areas, or if you're indoors.
- You might lack an internet connection. Your device may use information about
  GPS/GNSS satellites from the internet to [more quickly
  solve](https://en.wikipedia.org/wiki/Assisted_GNSS) for your location.
- You might be out of range of WiFi access points or cell towers. Your device
  may use [the presence of these
  signals](https://en.wikipedia.org/wiki/Wi-Fi_positioning_system), even if
  you're not connected to them, to determine your location with less accuracy
  than GPS/GNSS, but with better power efficiency.

## <a id="filters" href="#filters">Use Filters to Clean Up Data</a>

Your device's OS estimates of the accuracy of your location data, and this
estimate is stored along with each data point in your database. You can filter
locations based on this accuracy to hide inaccurate data from the map and the
timeline. Filters are accessed on the map tab, under the filter icon. 100
meters (330 feet) is the default accuracy filter, but we've found 20 meters (55
feet) to be another good balance between hiding inaccuracies, and not hiding
too much.

## <a id="versioning" href="#versioning">Scoria Versioning</a>

Scoria versions describe compatibility with other versions of Scoria. Versions
are broken down into three components: the major, minor, and patch version
numbers. At the time of this writing, the current version of Scoria is 1.4.2.
The major version number is 1, minor is 4, and patch is 2.

The meaning of the version numbers follows [Semantic
Versioning](https://semver.org/). The patch version number is incremented for
releases which do not affect compatibility. The minor version number is
incremented for releases that break compatibility with older versions of
Scoria. The major version number is incremented for releases that break
forwards compatibility, which should happen very rarely, if ever.

Currently, compatibility is defined by whether a Scoria database can be
imported into a different version of Scoria. Occasionally, new releases of
Scoria introduce changes to the way data is stored in your database. After one
of these updates, your database is migrated to the new format, and becomes
incompatible with an older version of the app. These database changes are
indicated by an increase in the app's minor version number, since they are
incompatible with older versions of the app.

Older databases can still be imported into a newer version of Scoria, since the
app knows how to update an old database. But an old app won't know how to deal
with a newer database format, since it doesn't know how the database changes in
the future.

For example, a database exported from Scoria 1.4.2 is compatible with versions
of Scoria that share the same minor number, such as 1.4.0, 1.4.1, and 1.4.3, as
well as versions that have a higher minor number, such as 1.5.0, and 1.6.0. But
the database is not compatible with app versions that have a smaller minor
number, like 1.2.0 or 1.3.0.
