# Single-tile parse probe (zoom 8, cmap)

2026-08-26. The parse-path check from memory-limits.md: with the
per-tile cap dropped, verify the worker pbf parse path has no cliff at
the densest tile the threshold knob can produce (max settable 100k
points per mount; tiles are served per mount, so multi-mount doesn't
stack features into one pbf).

`--mode tiles --zooms 8 --styles cmap --counts 100000,300000` (scatter).
The z12-viewport data straddles a z8 tile-column boundary, so N splits
across a few tiles rather than landing in exactly one; the 300k run's
max tile carries ~107k features (9.1 MB wire), which brackets the
knob-reachable single-tile worst case (100k in one tile ~ 8.5 MB) from
above. A true all-N-in-one-tile run needs --zooms 7 (boundaries move
with zoom); not needed while the knob tops out at 100k.

| N | data tiles | max tile MB | est max feat/tile | max tile ms | worker heap MB | crashed |
|---|---|---|---|---|---|---|
| 100k | 6 | 3.0 | 24.0k | 506 | 2.8 | no |
| 300k | 4 | 9.1 | 107.5k | 1447 | 5.1 | no |

Result: no cliff. Everything stays linear through the knob-reachable
regime: wire 84.6 B/feat, worker heap 11.2 B/visible-feat (5.1 MB at
the worst tile), page heap flat, no worker death. Max tile latency
1.45 s at 107k feat matches the known linear slice cost (~92 ms/10k
cmap) plus the O(N) scan floor at N=300k -- the latency knob's
territory, not a parse problem. Peak RSS 1.36 GB is the desktop
swiftshader upper bound seen in every tiles run, not a webview number.

Conclusion: no per-tile backstop needed anywhere the threshold knob can
reach; revisit only if the max settable threshold ever goes far above
100k (then probe --zooms 7 with N at the new max).

## z7 runs (2026-08-26, later): the All-preset regime

With TemporalMaxPoints::All landing, the knob range is no longer capped
at 100k, so the parse question reopened at large N. `--zooms 7` avoids
the z8 column straddle (a lat boundary still splits the data in two, so
the max tile carries ~N/2). The first 1M attempt was invalid -- the
probe returned on the empty first-pass tiles before the query/tileset
build finished; loadApp now waits for a data-bearing tile.

| N | max tile MB | est max feat/tile | max tile ms | worker heap MB | settled RSS d MB | crashed |
|---|---|---|---|---|---|---|
| 300k | 10.5 | 150k | 1648 | 4.6 | 588 | no |
| 1M | 34.9 | 500k | 5419 | 353.2 | 1222 | no |

No hard cliff: a 500k-feature, 35 MB single tile parses without a
crash or worker death, and latency stays linear (~110 ms/10k + scan
floor; 5.4 s self-inflicted All-tier latency). But steady-state worker
heap goes nonlinear past the old regime: ~11 B per visible feature
through 150k-feat tiles, ~350 B/feat at 500k (353 MB worker heap,
+1.2 GB settled RSS on the desktop rig) -- the worker retains far more
per feature on huge tiles than O(visible-tiles) intuition suggests.

Consequence: All is safe on desktops (its target), including the
memory. On a weak phone, All with several hundred k points zoomed out
could push the webview toward jetsam; the terminate handler turns that
into a reload, and the temporal presets remain the sane phone choices.
Noted in memory-limits.md; if phone-All ever matters, the options are
gating All off device memory or reviving a per-tile thinning backstop
sized ~200k+.
