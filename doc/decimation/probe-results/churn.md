<!-- Churn-mode results: in-page lifecycle leaks (memory-limits.md, "Webview resilience and leaks"). -->

Environment: same as baseline.md (M-series Mac, chrome-headless-shell 152,
swiftshader GL); 2026-08-26. Method: `--mode churn`, N=100k, style cmap,
walk shape, 12 cycles per kind on a live page, GC before each reading.
Cycle 0 is the post-load baseline. tabs = Log -> Map (map component
unmount + remount, map.remove() runs in the cleanup); pan = pan 1800 px
off the data and back (tile load/evict/reload + a backend query per
move).

## tabs: a real per-remount leak

| cycle | RSS MB | page heap MB | worker heap MB |
|---|---|---|---|
| 0 | 589.9 | 12.0 | 2.6 |
| 4 | 657.0 | 15.2 | 2.8 |
| 8 | 722.2 | 18.0 | 2.9 |
| 12 | 782.8 | 20.1 | 2.9 |

Late-cycle trend: RSS +14.8 MB/cycle, page heap +0.6 MB/cycle, worker
heap flat. Both slopes are linear through cycle 12 with no sign of a
plateau: every map unmount/remount leaks ~0.65 MB page heap and ~15 MB
RSS, despite map.remove() running in the component cleanup
(analyze.rs). about:blank leak mode showed clean teardown, so this is
app-level retention that page unload frees wholesale -- exactly the
blind spot churn mode was built for.

Suspected mechanism: the wasm callbacks created per map
(Closure::wrap(...).into_js_value() in new_map for load/moveend/idle,
plus get_click_point_callback and the pins callbacks) are leaked by
design -- into_js_value never frees the Rust closure -- and several
capture Rc<Map>. Each removed map's JS object graph therefore stays
reachable, including its canvas; with preserveDrawingBuffer: true a
1200x800 RGBA drawing buffer alone is ~3.8 MB, consistent with the
~15 MB native residue per map. Fix direction: own the Closures in the
component (drop them in the same cleanup that calls remove()), or hold
them in the map wrapper so they die with it; also null out
window.__stem_map on cleanup. Rerun churn tabs to verify.

Impact: ~15 MB per Map-tab visit on desktop; the phone webview pays
the same pattern (smaller surfaces, same retention). Tens of tab
switches in a session plausibly walks the webview into its jetsam
ceiling -- a candidate explanation for historically observed webview
reloads, independent of N.

## pan: high-water ratchet, no JS leak

| cycle | RSS MB | page heap MB | worker heap MB |
|---|---|---|---|
| 0 | 578.1 | 12.0 | 2.6 |
| 3 | 716.9 | 13.0 | 5.3 |
| 8 | 746.1 | 13.6 | 5.4 |
| 12 | 770.7 | 13.8 | 5.4 |

One-time step at cycle 3 (worker heap 2.6 -> 5.3 MB, RSS +120 MB), then
flat heaps: the tile cache and GL buffers reach a panning high-water
mark and hold it -- ratchet, not leak. The late RSS tail (+6 MB/cycle
with flat JS heaps) is native retention on the swiftshader rig; smaller
than the tabs signal and worth rechecking after the tabs fix, but not
alarming on its own. Tile data itself is released and reloaded
correctly across evict/refetch cycles.

## Root cause and fix (2026-08-26, follow-up)

The suspected mechanism above (leaked wasm Closures) was disproven:
owning the per-map Closures in a MapHandle dropped with the map
(mainmap.rs; verified active -- __stem_map clears on unmount, so
MapHandle::drop and map.remove() run) left the tabs slopes bit-for-bit
unchanged. Runtime.queryObjects then showed every removed maplibre Map
and its WebGL2 context still alive post-GC (5 of each after 4 cycles),
and a heap-snapshot retainer walk gave the chain:

Window -> window resize listener (registered by Plotly's
responsive:true) -> closure holding the #colorbar graph div -> detached
analyze-page DOM tree -> #map-div's maplibre listeners -> Map ->
_canvas -> WebGL2RenderingContext.

One un-purged plotly plot pinned the entire unmounted page; the map and
its ~15 MB GL context were collateral. Consistent with the Map<->Places
control run leaking identically (14.2 MB/cycle) with no Log page
involved. Fix: Plotly.purge bindings + purge destructors in the two
responsive:true plot effects (Colorbar in components/colorbar.rs,
ColoredTimeSeriesPlot in pages/metrics_dashboard.rs -- the latter also
covers the Stats page). The MapHandle ownership work is kept as real,
verified hygiene; it just wasn't the dominant term.

Verification (same parameters as the runs above):

| tabs cycle | RSS MB | page heap MB | worker heap MB |
|---|---|---|---|
| 0 | 587.5 | 11.9 | 2.6 |
| 4 | 550.7 | 11.9 | 1.4 |
| 8 | 550.8 | 12.0 | 1.4 |
| 12 | 551.0 | 12.0 | 1.4 |

Tabs late-cycle trend: 0.0 MB/cycle on all three signals (was +14.8
RSS / +0.6 page heap). Flat to 0.1 MB across 12 cycles; instance
counts confirm 1 map / 1 GL context / 6 canvases after cycling,
identical to fresh load. Pan stays a high-water ratchet (late RSS tail
+4.4 MB/cycle, below the pre-fix 6.0-9.2): its heap "slope" is a
measurement artifact -- the final cycle caught the worker mid-parse
(68.5 MB in flight), as GC'd readings race tile churn under load.
