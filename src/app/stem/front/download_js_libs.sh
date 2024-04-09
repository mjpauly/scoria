#!/bin/bash
set -euo pipefail
IFS=$'\n\t'

cd $BUILD_WORKSPACE_DIRECTORY/src/app/stem/front/static

curl https://cdn.plot.ly/plotly-strict-2.22.0.min.js -o plotly.min.js
curl https://unpkg.com/maplibre-gl@2.4.0/dist/maplibre-gl.js -o maplibre-gl.js
curl https://unpkg.com/maplibre-gl@2.4.0/dist/maplibre-gl.css -o maplibre-gl.css
