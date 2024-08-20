# Import Places from Google Maps

*Aug 20, 2024*

New in Scoria 1.4.2 is the ability to import places from a GeoJSON file. This
lets you migrate over places that you've saved in other apps.

If you export your saved places from Google Maps, your starred places will be
in a GeoJSON format and can be directly importing into Scoria.

However, all other lists are exported in CSV format, and lack the place
coordinates. To import these places, you'll need a way to get the coordinates
for each place, and turn the CSV into a GeoJSON.

We've created a tool for doing just this. It looks up the coordinates for each
place using the Google Maps URL, converting a CSV into a GeoJSON with the
location data. It can also fix up GeoJSON files, since some places might lack
coordinates even though they're in the GeoJSON.

## How-to

Once you've exported your data, you'll have one `Saved Places.json` for your
starred places and a collection of `.csv` files for all of your other lists.
You'll want to run `gmaps-coords` on each file, including the GeoJSON, to
ensure each place has coordinate data.

First, [install Rust](https://www.rust-lang.org/tools/install).

Next, install the `gmaps-coords` CLI tool, and a WebDriver server like
`geckodriver`. The WebDriver server lets `gmaps-coords` visit the Google Maps
webpage and retrieve each place's coordinates.

```shell
cargo install --git https://github.com/scoria-team/gmaps-coords.git
cargo install geckodriver
```

In one terminal, start `geckodriver`.

```shell
geckodriver
```

In a second terminal, run `gmaps-coords` on your files. The tool takes about
two seconds to look up each place's coordinates.

```shell
gmaps-coords -i saved_places.json -o saved_places_complete.json
gmaps-coords -i travel_list.csv -o travel_list_coords.json
```

### Parallelism

Multiple instances of the tool can be run at the same time using multiple
WebDriver instances. Specify the `-p` argument for `geckodriver` and
`gmaps-coords` to a value other than the default `4444`.

```shell
geckodriver -p 4445
gmaps-coords -p 4445 -i saved_places.json -o out.json
```

### More Options

```shell
gmaps-coords --help
```
