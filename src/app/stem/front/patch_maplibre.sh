#!/bin/bash
# Patch maplibre-gl.js to stencil-clip circle layers to their tile, like fill
# and line layers. Stock maplibre draws circles unclipped, so points near a
# tile edge overflow into the neighbor and overlap in tile order rather than
# feature order. With clipping, the backend adds a point buffer to each tile
# and every pixel is painted by exactly one tile in its feature order.
# Anchored on minified 4.7.1; each edit must match exactly once.
set -euo pipefail
f=${1:-maplibre-gl.js}

patch() {
    local from=$2 to=$3
    local n
    n=$(grep -cE "$from" "$f" || true)
    if [[ "$n" != "1" ]]; then
        echo "patch '$1': expected 1 match, found $n" >&2
        exit 1
    fi
    # in-place editing via a temp file, since sed -i differs BSD vs GNU
    sed -E "s/$from/$to/" "$f" > "$f.tmp"
    mv "$f.tmp" "$f"
}

# CircleBucket.addFeature: keep points in the tile buffer instead of
# dropping those outside [0, extent).
patch buffer \
    '(const r=e\.x,n=e\.y;)if\(r<0\|\|r>=[A-Za-z$_]+\|\|n<0\|\|n>=[A-Za-z$_]+\)continue;(const i=this\.segments\.prepareSegment\(4,)' \
    '\1\2'
# CircleStyleLayer.isTileClipped -> true so the painter renders the tile
# clipping masks before the layer.
patch clipped \
    '(createBucket\(t\)\{return new [A-Za-z$_]+\(t\)\})(queryRadius\(t\)\{const e=t;return [A-Za-z$_]+\("circle-radius")' \
    '\1isTileClipped(){return !0}\2'
# drawCircles: keep the per-tile stencil mode in the draw state...
patch stencil_state \
    '(uniformValues:[A-Za-z$_]+\(t,r,n,a\),terrainData:m)\}' \
    '\1,stencil:t.stencilModeForClipping(r)}'
# ...and draw with it instead of the disabled one.
patch stencil_draw \
    '(terrainData:l\}=e\.state;s\.draw\(h,c\.TRIANGLES,u),d,' \
    '\1,e.state.stencil,'
