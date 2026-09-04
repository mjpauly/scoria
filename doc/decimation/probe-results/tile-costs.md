<!-- Per-tile cost probe results (memory-limits.md, "MVT tile serving"). -->

Environment: same as baseline.md (M-series Mac, chrome-headless-shell 152,
swiftshader GL); 2026-08-25. Method: `--mode tiles` in memory_probe.mjs.
Synth data (scatter shape) always fills the z12 viewport; `--view-zoom`
repacks the same N into fewer, denser tiles (z10 puts everything in ~4
tiles here; the data bbox straddles tile boundaries at this center, so a
z10 "one tile" run actually lands ~N/3 in the max tile). est max
feat/tile apportions N by tile wire bytes. Tile ms is request-to-finish
seen from the maplibre worker: backend slice CPU + blocking-pool queue +
localhost transfer.

| style | N | zoom | data tiles | mvt MB | max tile KB | est max feat/tile | max tile ms | med tile ms | first idle ms | worker heap MB | page heap MB | settled RSS d MB |
|---|---|---|---|---|---|---|---|---|---|---|---|
| points | 100000 | 12 | 24 | 4.4 | 356.9 | 8126 | 25 | 11 | 2083 | 1.7 | 10.7 | 408.8 |
| points | 100000 | 10 | 8 | 4.4 | 840.1 | 19127 | 53 | 51 | 2173 | 1.9 | 10.7 | 403.6 |
| points | 300000 | 12 | 12 | 6.6 | 1074.4 | 48933 | 72 | 33 | 1017 | 2.7 | 10.7 | 440.0 |
| points | 300000 | 10 | 4 | 6.6 | 2515.4 | 114615 | 153 | 150 | 1018 | 2.6 | 10.7 | 444.6 |
| lines | 100000 | 12 | 24 | 20.6 | 1563.3 | 7591 | 115 | 47 | 2507 | 3.1 | 10.7 | 532.8 |
| lines | 100000 | 10 | 8 | 13.0 | 2327.4 | 17897 | 144 | 142 | 2093 | 2.7 | 10.7 | 463.2 |
| lines | 300000 | 12 | 12 | 30.8 | 4683.6 | 45555 | 368 | 303 | 1026 | 6.4 | 10.8 | 687.2 |
| lines | 300000 | 10 | 4 | 19.5 | 6986.6 | 107602 | 425 | 418 | 1024 | 4.9 | 10.7 | 534.4 |
| cmap | 100000 | 12 | 24 | 27.0 | 2050.4 | 7584 | 354 | 150 | 2645 | 3.3 | 12.0 | 603.7 |
| cmap | 100000 | 10 | 8 | 17.3 | 3101.3 | 17945 | 851 | 475 | 2151 | 2.8 | 11.9 | 521.4 |
| cmap | 300000 | 12 | 12 | 40.5 | 6159.6 | 45671 | 978 | 467 | 1091 | 6.6 | 12.0 | 801.5 |
| cmap | 300000 | 10 | 4 | 25.9 | 9299.3 | 107824 | 1438 | 1424 | 1091 | 5.1 | 11.9 | 629.4 |

## Per-style rates (regression across runs)

wire B/feat from the densest run (in-tile rate, free of cross-tile
segment duplication); heap and RSS per visible feature regress on N;
tile latency regresses on est max feat/tile across the zoom x N grid.

| style | wire B/feat | worker heap B/feat | settled RSS B/feat | tile ms per 10k feat |
|---|---|---|---|---|
| points | 21.9 | 4.5 | 180.5 | 11.3 |
| lines | 64.9 | 13.9 | 564.1 | 31.4 |
| cmap | 86.2 | 13.8 | 764.2 | 91.5 |

## Findings

- Browser memory is confirmed a non-issue in the MVT architecture at any
  measured density: worker heap is 4-14 B per visible feature (6.6 MB max
  with all 300k features on screen), page heap flat. No frontend memory
  budget is needed; the per-tile cap is sized by latency and wire bytes.
- Tile latency is the binding cost and is style-dependent: ~11 (points) /
  ~31 (lines) / ~92 (cmap) ms per 10k features in the tile. The cmap rate
  is dominated by per-feature color-tag encoding, not geometry. Worst
  measured: cmap at ~108k feat/tile = 1.44 s and 9.3 MB wire per tile.
- Latency has an O(N) scan floor independent of tile content: at 300k,
  even z12 tiles holding ~46k features run 300-470 ms med (lines/cmap)
  because slice_tile scans the whole working set per tile. The planned
  coarse spatial bin over the working set attacks this floor; a feature
  cap alone does not.
- Wire duplication across tiles only matters for segments: points total
  mvt bytes are identical at z12 and z10 (4.4 MB at 100k), while lines
  and cmap shrink 30-40% at z10 (fewer boundary-straddling segments
  duplicated, fewer per-tile layer headers).
- first idle ms is not a per-tile jank signal: the map reports idle
  before slow tiles arrive (1.0-1.1 s at 300k while cmap tiles took up
  to 1.4 s). Treat tile ms as the responsiveness metric.
- Cap recommendation: a per-tile feature cap around 30-50k keeps the
  worst style (cmap) near 0.3-0.5 s per tile and ~3-4 MB wire, while
  points-only could afford 200k+. Since tiles slice in parallel, the
  viewport refresh cost is roughly the max tile's latency, so the cap
  bounds refresh latency directly. Below ~30k the cap would start
  binding at z12 densities the current 10k global limit already allows
  onto one screen, so 30-50k is the useful range; enforcement (thinning
  in slice_tile, or the spatial pitch pipeline) would be the next step.
  (The cap was later dropped: every cost here is linear and the knob
  range bounds in-tile count -- see memory-limits.md.)
