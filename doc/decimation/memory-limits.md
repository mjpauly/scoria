# Map decimation and memory limits

August 2026 redesign of the map data path: the hand-tuned global 10k
point cap became user decimation settings backed by a vector-tile
serving path, a spatial grid index, and a never-bind backend memory
backstop. This doc records the shipped design and the measurements
behind each decision. Per-experiment write-ups are in probe-results/.

## The reframe that drove the design

The 10k cap was protecting webview memory: with a geojson source,
browser memory ran ~50x the geojson string (the maplibre worker's
geojson-vt tile pyramid), so 10k points already cost 60-90 MB of
worker heap and 300k could kill the worker near 2 GB
(probe-results/baseline.md). Switching the map source to server-sliced
vector tiles (probe-results/k-reduction-ab.md, "MVT source prototype")
made browser memory O(visible tiles) instead of O(N): worker heap
dropped 100-300x, nearly flat in N, and every style was stable at
300k. With that, latency binds before memory everywhere measured, so
decimation stopped being crash prevention and became
latency-vs-detail knobs with reasonable defaults, plus cheap
never-bind backstops. (An earlier k-reduction pass on the geojson
path -- merging consecutive equal-properties features, source
buffer=0 -- won 80-93% of worker heap; MVT superseded the merging,
and buffer=0 stays baked in for the remaining geojson sources.)

## MVT tile serving

make_tileset (back/map/mvt.rs) stores each mount's query result as a
TileSet working set (~80 B/pt with lines: 32 B/pt + 48 B/seg tuples;
cmap colors are interned); slice_tile serves tiles on demand. The
geojson map path is gone (mount_geojsons, the .geojson route, the
front geojson sources); MapQueryStats reports n_segs + tileset_bytes.

- Slicing runs on the blocking pool with the tileset behind RwLock +
  Arc, so concurrent tile requests slice in parallel (serialized they
  queued up to ~300 ms per tile).
- Query changes reload live tiles in maplibre's "expired" state
  (reload_source_expired in mainmap.rs, a hand-rolled refreshTiles
  from maplibre >= 5.6 against the vendored 4.7.1) -- no flicker, and
  a real refetch rather than reload()'s re-parse of cached pbf. Tile
  responses are no-store, so no cache-busting URL param.
- Antimeridian segments are cut in the tileset build. Segments with a
  far off-tile endpoint are Liang-Barsky clipped to the tile plus a
  256/4096 buffer (maplibre's geometry loader clamps coordinates to
  +-4 tile widths, which bent long segments at tile boundaries when
  zoomed in).
- Source maxzoom is 20 (~9 mm quantization; 14's ~0.5 m gave ~1 px of
  aliasing at z17, and per-tile slicing cost is zoom-independent).
- Tilesets are evicted on unmount and cleared on app backgrounding
  (rebuilt by the foregrounding update).

Measured per-tile costs (probe-results/tile-costs.md): slice latency
~11/31/92 ms per 10k in-tile features for points/lines/cmap, worker
heap 4-14 B per visible feature, plus an O(N) floor from slice_tile
scanning the whole working set per tile request (~300-470 ms at
N=300k; a coarse spatial bin over the working set is the eventual
fix). There is deliberately no per-tile feature cap: every per-tile
cost is linear through the densest reachable tiles, and the worker
parse path has no cliff even at a 500k-feature 35 MB single tile
(probe-results/single-tile-parse.md) -- though steady-state worker
heap goes nonlinear past ~150k feat/tile (~350 B/feat at 500k), so a
weak phone zoomed out on several hundred k points could still jetsam
the webview; the terminate handler turns that into a reload.
Tile-level thinning was also rejected on quality grounds: silent, and
non-uniform (adjacent tiles thinned at different rates put density
steps at tile boundaries).

## Decimation settings

DecimationMode + TemporalMaxPoints (common/map_style.rs), a mode
select plus per-mode knob in map_settings.rs. The old
DECIMATION_THRESHOLD static is gone; decimation_threshold(&style)
reads the setting per query (the STEM_DECIMATION_THRESHOLD env
override is kept for the probe harness).

- Spatial (default, 0.5 px pitch): bucketed newest-per-cell query,
  the better look when viewing everything. On movement-triggered
  track-like data, temporal thinning is uniform per unit time and
  erases dwells; spatial keeps one point per cell per dwell
  (probe-results/cell-occupancy.md). Bucketing kicks in over the
  fixed SPATIAL_BUCKETING_TRIGGER (10k, map_data.rs), checked with a
  LIMIT-bounded count (O(trigger) rows, 2-20 ms).
- Temporal (presets 5k-100k, default 10k): id-stride decimation. Kept
  for fast queries and for lines: lines need the time-adjacent points
  just outside the viewbounds, and fetch_adjacent locates the gaps
  via the id stride of temporal decimation, which bucketed results
  lack.
- A TemporalMaxPoints::All preset (backstop as the only limit) was
  added and then dropped after the phone pass: spatial is the way to
  see detail.

iPhone 16 Pro pass, ~1.1M points in view: spatial 0.5 px returned 75k
points in 1.6 s (4.2 MB tileset); temporal 100k queried in 0.5 s but
looked visibly thinner; temporal All (1.1M points, 67 MB tileset, 4 s
query) was barely usable. That pass fixed the defaults above.

## Spatial grid index

The bucketed query was an O(points in view) scan + GROUP BY (1.4-2.4 s
at 1-2M points in view). The data is track-like -- occupied cells
collapse linearly with cell width, with no jitter scale at which it
steps (cell-occupancy.md) -- so no single cell size culls it; what
works is a (cell, timestamp) index per zoom level, turning "newest
point per cell" into a loose index scan over occupied cells
(probe-results/cell-index-bench.md and cell-index-levels.md, with the
level-choice sweep and plot).

Shipped: migration 20260827000000_grid_cells adds u32 Mercator grid
columns gx/gy (computed in Rust at insert) and virtual cells cy/cx at
levels 8/10/12 (0.5 px cells at those view zooms; finer levels drown
in the row fetch, coarser ones don't earn their ~16% disk each).
database::open_db backfills existing rows (database/grid.rs,
resumable 4k-row batches) before 20260828000000_grid_indexes builds
the three (cy, cx, timestamp) indexes (+7% of db size for gx/gy,
~16% per index). Opens run in the background (status in
DerivedState::mounted_dbs_on_disk) and a database's pool is only
published once fully migrated, so queries never see a partial grid;
locations logged meanwhile are queued and inserted before publish. On
2.5M rows the whole migration runs ~24 s in the background.

FilteredQuery::fetch_bucketed_grid picks the coarsest index not
coarser than the query cell (a recursive-CTE walk grouping index
cells into query cells); for query cells finer than z12 the z8 index
serves as a 2-D filter. Time range and user filters apply in the
per-cell newest-row seek; pitch is quantized to grid-aligned powers
of two (SpatialCell::grid_shift). Verified by
test_grid_bucketed_matches_group_by (identical to a GROUP BY
reference across matched/coarser/finer levels with bounds, time
range, filter, and cap), test_grid_query_plan (every table access via
the level index or the rowid join), and test_grid_backfill. On the
2.56M-point db: 70-90 ms vs 1.4 s (15-20x) at z6-z10 pitches; a full
dev-server sweep over map zooms 1-17 (debug build, desktop viewport)
gave 4-6x up to z10, 2-3x at z11-12, ~1x from z14 where the per-cell
row fetch dominates (~140k cells on a 1400x900 screen), never worse.
A narrower row fetch (only tileset columns) is the remaining lever
for city zooms.

Query routing (probe-results/time-range-bench.md, implemented in
fetch_decimated_result_with_db): time-restricted queries use the
timestamp autoindex -- the old likelihood(..., 1.0) planner pins are
gone. Spatial mode takes the time route under TIME_PATH_MAX_ROWS
(300k rows in range, LIMIT-counted on the index; ids need not be
time-ordered after an import), else the grid walk (~3-4 us per
occupied cell in view, independent of time range). Temporal mode runs
two LIMIT-bounded index-only counts (TEMPORAL_COUNT_CAP, 1M) -- rows
in range vs all-time points in view -- and the smaller side wins.
lngtimeindex is dropped by the grid_indexes migration: within ~30% of
the grid or slower on every measured shape.

## Backend memory backstop

Backend OOM kills the app process, which no reload handler can save,
so a never-bind cap exists even though latency binds far earlier
(sized limits land in the millions of points on a weak phone; only a
huge import or multi-mount pileup reaches them).
FilteredQuery::hard_cap clamps the temporal decimation limit and
bounds bucketed fetches to the most recent hard_cap cells (ORDER BY
max(timestamp) DESC LIMIT cap, re-sorted ascending in Rust) -- the
backstop and the recency guard for the tiny-pitch cluster pathology
are one mechanism. backend_backstop_points (map_data.rs) sizes it:
budget = physical RAM / 4 (hw.memsize on Darwin, /proc/meminfo on
Android/Linux, 2 GB weak-device fallback, cached in a OnceLock),
split across enabled mounts, divided by the live
tileset_bytes/n_points rate from the last query when n_points >= 1k
(fallback and floor 80 B/pt), then floored at 100k so it can never
undercut the knob. DecimatedResult and MapQueryStats carry a
truncated flag surfaced in the settings stats line, so the backstop
cannot bind silently; get_location_near passes the same cap.
test_hard_cap (location.rs) asserts it binds only on the cluster
pathology and on a sub-limit cap.

## Webview resilience and leaks

- A dead webview renderer reloads the page: iOS via
  webViewWebContentProcessDidTerminate (ios/top/ViewController.swift),
  Android via onRenderProcessGone, which must destroy the dead WebView
  and build a fresh one (android/kt/CustomWebViewClient.kt). Backend
  state lives in the app process, so the page comes back to the same
  view.
- Load/unload leak mode is clean on the MVT path
  (probe-results/leak-mvt.md): JS heaps flat across 12 cycles, tile
  data fully released, RSS a decelerating native ratchet.
- In-page churn testing (probe-results/churn.md) found the real leak:
  Plotly's responsive:true window resize listener retained the whole
  detached analyze page, ~15 MB RSS per Map-tab visit -- a candidate
  explanation for historically observed webview reloads. Fixed with
  Plotly.purge destructors in Colorbar and ColoredTimeSeriesPlot;
  per-map wasm Closures are owned by a MapHandle dropped with the map
  (kept as verified hygiene; measured not to be the leak). The purge
  sites capture the element in the effect and purge under a catch
  binding: yew runs a child's effect destructors after the parent DOM
  is detached, and an id lookup throwing through wasm there poisons
  the hook context and blanks every later render.
- The churn modes (tabs, pan) remain the regression check; pan is a
  clean high-water ratchet, not a leak.

## Probe harness

The dev binary with --synth N --shape S --style C [--view-zoom Z]
bulk-inserts synthetic data (walk/scatter/cluster shapes) into a
fresh db and seeds probe state (analyze route, viewport, style, wide
time range). dev/memory_probe.mjs drives chrome-headless-shell over
raw CDP (auto-installed; the /Applications ungoogled-chromium hangs
headless) with modes: sweep (memory vs N), tiles (per-tile costs),
leak (load/unload cycles), churn (tabs/pan lifecycles). Wasm memory
shows in the in-app stats line -- the one memory signal available
inside a WKWebView. Committed results are in probe-results/. Not a CI
gate; a regression mode against the committed baselines remains an
option.

Caveat on the recorded desktop latencies: dev-server and probe runs
used bazel fastbuild, where libsqlite3-sys compiles SQLite at -O0 and
the Rust slice path is unoptimized; the iOS build is -c opt. Query
and tile-ms numbers in probe-results are therefore pessimistic.

## Remaining work

### zoom-all bounds query (fixed 2026-09-03: on-demand)

`update_zoom_all_data` (map_data.rs) used to run after every
completed map update, including pan-only updates and the forced
foreground update that runs even off the Analyze route (core.rs
`update_map_data(None, true)` skips the route check). Its
`fetch_bounds` (location.rs `get_bounds_query`) is `SELECT
MIN/MAX(longitude/latitude)` over the time range + filters, and no
index covers those columns, so it scans every row in range: 745 ms
per update on the real dataset (fastbuild desktop). On a multi-GB
database that scan reads a large fraction of the file per update; it
was what filled the (then 2 GiB) page cache within seconds of launch
during the 2026-09-03 scrub-crash investigation. With the
per-connection cache caps it was no longer a memory problem, only
wasted IO/CPU/battery and a hot connection per update.

Fix: the query now runs only on button press. The button sends
`Request::DataViewParams` over the ws request/response channel;
the back's `get_data_view_params` (map_data.rs) runs `fetch_bounds`
and returns (center, zoom), and the front flies there. `data_center`
left BackState. Changing the time range is far more common than
pressing zoom-all, so on-demand beats memoizing on (time_range,
filters); the press eats the scan latency (logged at debug as
`zoom-all fetch_bounds`).

If that latency ever matters, the ideas from the eager era still
apply, cheapest first:

1. Memoize by (time_range, filters) for repeated presses.
2. Serve it from the grid: gx/gy columns and the (cyL, cxL,
   timestamp) cell indexes already exist. A loose index scan over
   occupied z8 cells gives min/max cell coordinates; only the four
   extreme edge cells need row reads to make the bounds exact. Same
   recursive-CTE machinery as fetch_bucketed_grid, O(occupied cells)
   instead of O(rows). Complication: the cell indexes ignore filters,
   so filtered queries either fall back to the scan or accept
   filter-less bounds (zoom-all arguably wants the unfiltered extent
   anyway -- decide then).
3. Maintain per-day min/max side stats on insert/import and aggregate
   over the range, O(days). More moving parts (import, delete,
   backfill), so only if measured to matter.

Done since: the view-window fetch expansion (the geojson-cut leftover)
became a fixed 10 px pad (MARKER_SIZE_MAX, the widest marker) at the
view zoom, replacing the cruder 4% of view width; the narrow row fetch (NarrowPoint; see
probe-results/grid-walk-alternatives.md) roughly halved large fetches,
and the temporal default moved to 100k, calibrated on an iPhone 16 Pro
(~800 ms per 100k retrieved pre-narrowing, ~half with it; the probed
single-tile worst case covers 100k-in-one-tile). Presets above 100k
would need a --zooms 7 single-tile re-probe: 150k feat/tile is still
in the linear ~11 B/feat regime, but 200k+ is unmeasured between there
and the nonlinear ~350 B/feat seen at 500k.

## Considered and rejected, or deferred

- Per-tile feature cap / tile-level thinning: rejected, see the MVT
  section. Revisit a ~200k+ thinning backstop only if All-scale
  tiles on phones ever matter.
- Probe-and-coarsen spatial enforcement: sampled two-width
  count(DISTINCT cell_key) probes on the count query, a local scaling
  exponent d, one-shot (c1/T)^(1/d) width coarsening. Retired when
  pitch became a pure quality setting with no memory limit to
  enforce.
- R-tree: answers containment, not newest-per-cell; O(N in view)
  like the scan (see the note in cell-index-levels.md).
- u8 cmap levels in the TileSet (frontend maps level -> color in a
  style expression): modest memory win (~20%, 80 -> 64 B/pt) but
  likely the cheapest big lever on cmap slice latency, the worst
  per-tile cost -- mvt-0.8.1's add_tag_string dedupes by
  linear-scanning the layer value table per feature (~128 string
  compares average at 256 levels), the plausible mechanism for
  cmap's 3x latency over lines. Pre-interned indexed values skip
  that, and colormap switching becomes a pure restyle. Needs the
  click readout to map level -> color front-side.
- Fixed-point u32 coordinates (2^-32 of world ~ 9 mm; f32's ~2.4 m
  worst case is unacceptable): TileSet drops to ~32 B/pt and integer
  tile math might dent the slice-scan floor. Only interesting if
  that floor matters before a coarse spatial bin lands.
- Coarse spatial bin over the TileSet working set: the fix for the
  O(N) slice_tile scan floor, when it matters.
- Precomputed coarse levels (mipmap side tables maintained on
  insert): invalid under time-range and user filters; the grid index
  covers the need.
- Persist tilesets to disk on backgrounding (instant foreground
  render while the query runs): deferred; staleness vs the live db
  and invalidation complexity outweigh the nice-to-have.
- Previous-query feedback anchor for pitch: likely unnecessary.
