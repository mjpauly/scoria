<!-- A/B results for the k-reduction experiments on the geojson map path, and the MVT prototype that superseded them (memory-limits.md, "The reframe that drove the design"). -->

Environment: same as baseline.md (M-series Mac, chrome-headless-shell 152,
swiftshader GL); 2026-08-24/25. Method: each arm hardcodes one knob
(GEOJSON_MAXZOOM/BUFFER/TOLERANCE consts in front/rs/maplibre/mainmap.rs,
MERGE_SOLID_FEATURES in back/map/geojson.rs), rebuilds, and reruns the
memory probe sweep at N={30k,100k}, walk shape. "defaults" is the scaffolding sanity arm
(explicit maplibre defaults; matches control within noise). combo =
merge + buffer0, run out to N=300k. "vs ctrl" compares at equal style and N;
control had no 300k runs (300k was unstable in baseline).

| arm | style | N | geojson MB | worker heap MB | vs ctrl | settled RSS d MB | vs ctrl | flags |
|---|---|---|---|---|---|---|---|---|
| control | points | 30000 | 3.5 | 173.6 |  | 616.8 |  |  |
| control | points | 100000 | 11.7 | 549.5 |  | 1148.3 |  |  |
| control | lines | 30000 | 8.4 | 265.1 |  | 726.2 |  |  |
| control | lines | 100000 | 28.1 | 726.2 |  | 1484.1 |  |  |
| buffer0 | lines | 30000 | 8.4 | 137.5 | -48% | 585.6 | -19% |  |
| buffer0 | lines | 100000 | 28.1 | 405.2 | -44% | 1150.5 | -22% |  |
| combo | points | 30000 | 1.2 | 22.6 | -87% | 398.9 | -35% |  |
| combo | points | 100000 | 4.0 | 47.2 | -91% | 447.4 | -61% |  |
| combo | points | 300000 | 12.0 | 108.4 |  | 556.0 |  |  |
| combo | lines | 30000 | 2.4 | 24.7 | -91% | 416.2 | -43% |  |
| combo | lines | 100000 | 8.0 | 52.3 | -93% | 489.6 | -67% |  |
| combo | lines | 300000 | 24.0 | 179.6 |  | 692.1 |  |  |
| defaults | points | 30000 | 3.5 | 173.1 | -0% | 617.3 | 0% |  |
| defaults | lines | 30000 | 8.4 | 248.5 | -6% | 715.5 | -1% |  |
| merge | points | 30000 | 1.2 | 35.3 | -80% | 413.0 | -33% |  |
| merge | points | 100000 | 4.0 | 85.3 | -84% | 494.9 | -57% |  |
| merge | lines | 30000 | 2.4 | 40.0 | -85% | 436.6 | -40% |  |
| merge | lines | 100000 | 8.0 | 98.9 | -86% | 569.1 | -62% |  |
| mz10 | points | 30000 | 3.5 | 153.5 | -12% | 585.0 | -5% |  |
| mz10 | points | 100000 | 11.7 | 495.4 | -10% | 1076.5 | -6% |  |
| mz10 | lines | 30000 | 8.4 | 204.3 | -23% | 671.3 | -8% |  |
| mz10 | lines | 100000 | 28.1 | 522.8 | -28% | 1205.9 | -19% |  |
| mz12 | points | 30000 | 3.5 | 180.8 | 4% | 616.1 | -0% |  |
| mz12 | points | 100000 | 11.7 | 587.7 | 7% | 1175.4 | 2% |  |
| mz12 | lines | 30000 | 8.4 | 240.8 | -9% | 715.9 | -1% |  |
| mz12 | lines | 100000 | 28.1 | 709.2 | -2% | 1412.1 | -5% |  |
| mz14 | points | 30000 | 3.5 | 178.8 | 3% | 615.8 | -0% |  |
| mz14 | points | 100000 | 11.7 | 588.7 | 7% | 1178.3 | 3% |  |
| mz14 | lines | 30000 | 8.4 | 252.0 | -5% | 729.4 | 0% |  |
| mz14 | lines | 100000 | 28.1 | 720.5 | -1% | 1409.4 | -5% |  |
| tol1 | lines | 30000 | 8.4 | 213.3 | -20% | 677.6 | -7% |  |
| tol1 | lines | 100000 | 28.1 | 597.0 | -18% | 1309.7 | -12% |  |


## Findings

- Feature merging (one MultiPoint / MultiLineString per query instead of a
  feature per point) dominates: -80..86% worker heap at equal N, and it also
  shrinks the geojson string ~3x (r_pt ~40 B/pt merged vs 117 unmerged).
  Net memory per point is ~14x lower than control.
- buffer=0 is the second lever for lines (-44..48% alone) and stacks with
  merging: combo is -91..93% worker heap vs control.
- tolerance=1.0 gives -18..20% on lines (not included in combo; visible
  simplification tradeoff, revisit if more headroom is needed).
- geojson source maxzoom is NOT the big lever (contrary to the guess in the
  plan): 14 and 12 are noise-level, 10 gives only -10..28%. The geojson-vt
  pyramid is built on demand, so depth barely matters while the map sits at
  one zoom.
- Combo absolute anchors: 100k pts = 47 MB worker heap (vs 549 control);
  300k lines = 180 MB, stable, where the control config's worker died near
  2 GB in baseline runs. Combo k (heap-per-string-byte slope): ~8 points-only,
  ~7 lines, vs ~50 / ~24 control -- but note the string itself also shrank.

## Caveats before shipping the combo

- get_click_point_callback (mainmap.rs) reads feature.geometry.coordinates
  as one lng/lat pair; a merged MultiPoint breaks that. Fix: use the click
  event's lngLat (get_location_near does the nearest-point lookup anyway).
- Visual QA (done 2026-08-25, desktop dev app): merged rendering looks
  correct, and buffer 0 vs 512 is pixel-identical in screenshot comparison.
  Expected for these styles: circles are not stencil-clipped (the owning
  tile draws the full marker), and lines are stencil-clipped with butt caps
  (each pixel painted exactly once either way). Buffer would only matter for
  round/square caps, dashes, or line gradients. Pre-existing tile-boundary
  visibility is cross-tile stacking order, unrelated to buffer. buffer=0 is
  safe as the shipped value.
- cmap styles originally kept the per-feature path; superseded by the
  run-merge follow-up below, which extends merging to cmap via same-color
  runs.

## Cmap run-merge (2026-08-25 follow-up)

Merging generalized from solid-only to runs of consecutive equal-properties
features (MERGE_FEATURE_RUNS replaces MERGE_SOLID_FEATURES; solid styles
are the properties-None case, so one code path covers all styles).
Colormaps have 256 levels, so autocorrelated datastreams produce long
same-color runs. The synth generator gained STEM_SYNTH_SPEED=cruise (speed
constant over 30-300 sample segments) as the realistic case; the default
i.i.d. speed is the worst case, since consecutive samples almost never land
in the same 1-of-256 bin (runs of ~1). Style cmap, page+worker heap MB:

| arm | data | N=30k | N=100k | geojson MB at 100k |
|---|---|---|---|---|
| control | cruise | 250.0 | 705.4 | 31.5 |
| merged | cruise | 36.6 (-85%) | 85.6 (-88%) | 8.2 |
| control | iid | 246.1 | 724.2 | 31.5 |
| merged | iid | 239.9 (-3%) | 677.4 (-6%) | 32.8 |

- On realistic data the cmap win matches the solid-style win. The i.i.d.
  floor is neutral (string +4% from MultiPoint-of-one wrappers, heap within
  noise), so run merging has no pathological downside; heading on curvy
  roads just gets no benefit.
- Solid sanity rerun through the unified path reproduces the earlier merge
  arm: 36.3 / 40.5 MB at 30k points / lines.
- Draw order is preserved: runs flush in chronological order, so
  newest-on-top stacking is unchanged (unlike a one-feature-per-color-level
  grouping, which would restack overlaps by color).
- The color property rides on each run feature, so ["get","color"] paint
  and the click color readout work unchanged; the click-handler geometry
  caveat above is the same for cmap runs.
- The first i.i.d. 100k attempts timed out loading (31 MB string vs the
  probe's 240 s window); probe load timeout raised to 480 s.
- Real-data merge ratios (2026-08-25, user's data, 127,502 records,
  points only): speed cmap merges to 95,387 features (ratio 0.75, near the
  i.i.d. floor -- real speed jitter spans several of the 256 bins); time
  cmap merges to 222 features (ratio 0.002, effectively full merge). This
  confirms sizing limits with worst-case k per style: cmap-by-speed sits
  near unmerged k, cmap-by-time and solid styles get the full merged win.

## MVT source prototype (2026-08-25)

Same query/decimation/cmap pipeline, but the result is stored as a working
set (back/map/mvt.rs) and served as vector tiles sliced on demand;
maplibre uses a vector source (USE_MVT_SOURCE in mainmap.rs) instead of
geojson. Browser memory becomes O(visible tiles), not O(N): the string,
parse tree, and client-side geojson-vt pyramid all disappear. Worker heap
MB (geojson controls from the same-session arms; 300k from baseline runs):

| style | N | mvt fetched MB | worker heap MVT | worker heap geojson |
|---|---|---|---|---|
| points | 30k/100k/300k | 1.3 / 4.4 / 6.6 | 1.5 / 1.8 / 2.7 | 163 / 539 / 1743 |
| lines | 30k/100k/300k | 3.0 / 10.0 / 15.0 | 1.7 / 2.5 / 30.7 | 254 / 715 / 2072 (unstable) |
| cmap | 30k/100k/300k | 4.2 / 13.8 / 20.5 | 1.9 / 2.6 / 4.7 | 231 / 708 / (unstable) |

- Worker heap is ~100-300x smaller and nearly flat in N (300k lines'
  30.7 MB is the one mild outlier, still 67x under geojson). Page heap is
  flat at baseline. Peak RSS <= 1.1 GB at 300k vs 3.2-4.4 GB geojson;
  settled RSS improvements are muted on this rig by the fixed ~380 MB
  swiftshader surface floor (heap is the portable signal).
- Unmerged features with per-feature color tags: the speed-cmap
  merge-ratio problem is moot in this architecture, since per-feature cost
  is paid only for visible tiles. Cmap wire size is ~68 B/pt of tiles
  fetched at a fit-all viewport.
- No decimation-limit formula is needed for browser memory in this mode;
  what remains memory-relevant is points-per-viewport (spatial decimation
  pitch), and tile slicing gives every zoom its natural pitch.
- Prototype caveats (resolved 2026-08-25 unless noted: visual QA passed,
  antimeridian segments now cut in the tileset build; geojson strings are
  still built redundantly alongside the tileset until the geojson map
  path is dropped); source
  maxzoom 14 quantizes positions to ~0.6 m on overzoom; query changes
  refresh in place: live tiles reload in maplibre's "expired" state (the
  refreshTiles mechanism from maplibre >= 5.6, hand-rolled for the
  vendored 4.7.1), so old tiles keep rendering until replacements arrive
  -- no flicker, and a real refetch rather than reload()'s stale re-parse
  of cached pbf. Tile responses are no-store, so no cache-busting URL
  param is needed.
