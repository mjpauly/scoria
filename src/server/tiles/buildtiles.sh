#!/bin/bash
set -euo pipefail
IFS=$'\n\t'

# get osmium for extracting an area from the osm data
HOMEBREW_NO_AUTO_UPDATE=1 brew install osmium-tool

mkdir build
cd build

# get tilemaker (-L follows github's redirect)
curl -L -O https://github.com/systemed/tilemaker/releases/download/v2.4.0/tilemaker-macos-10.15.zip
# unzip into tilemaker/
unzip -d tilemaker tilemaker-macos-10.15.zip
chmod +x tilemaker/build/tilemaker


# get osm data
mkdir osm-data
cd osm-data
curl -O https://download.geofabrik.de/north-america/us-west-latest.osm.pbf
cd -
curl -O https://osmdata.openstreetmap.de/download/water-polygons-split-4326.zip
# coastline/ needs to be visible from where we run tilemaker, we don't explicitly point it there
unzip -d coastline water-polygons-split-4326.zip

# cut out desired area (small is better for testing)
osmium extract --bbox=-123.13,36.4,-120.92,38.84 --set-bounds --strategy=smart osm-data/us-west-latest.osm.pbf --output osm-data/bayarea.osm.pbf

# make the mbtiles
mkdir mbtiles-data
./tilemaker/build/tilemaker --input osm-data/bayarea.osm.pbf --output mbtiles-data/bayarea.mbtiles --process tilemaker/resources/process-openmaptiles.lua --config tilemaker/resources/config-openmaptiles.json
