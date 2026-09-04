<!-- Leak-mode results against the MVT path (memory-limits.md, "Webview resilience and leaks"). -->

Environment: same as baseline.md (M-series Mac, chrome-headless-shell 152,
swiftshader GL); 2026-08-25. Method: leak mode, N=100k, style cmap (the
heaviest tile path), walk shape, 12 load/unload cycles, GC before each
reading. Leak-mode byte accounting now counts mvt bytes.

| cycle | loaded RSS MB | loaded page heap MB | loaded worker heap MB | unloaded RSS MB | unloaded page heap MB |
|---|---|---|---|---|---|
| 0 | 581.6 | 12.0 | 2.6 | 468.3 | 0.7 |
| 1 | 593.8 | 11.8 | 2.6 | 479.7 | 0.7 |
| 2 | 598.2 | 11.8 | 2.6 | 485.7 | 0.7 |
| 3 | 600.6 | 11.8 | 2.6 | 488.2 | 0.7 |
| 4 | 601.1 | 11.9 | 2.6 | 490.7 | 0.7 |
| 5 | 604.9 | 11.9 | 2.6 | 493.1 | 0.7 |
| 6 | 608.4 | 11.9 | 2.6 | 494.2 | 0.7 |
| 7 | 607.3 | 11.9 | 2.6 | 495.0 | 0.7 |
| 8 | 611.7 | 11.9 | 2.6 | 496.9 | 0.7 |
| 9 | 611.8 | 11.8 | 2.6 | 498.1 | 0.7 |
| 10 | 612.8 | 11.8 | 2.6 | 498.8 | 0.7 |
| 11 | 610.1 | 11.9 | 2.6 | 499.7 | 0.7 |

Late-cycle trend: unloaded RSS 1.2 MB/cycle, unloaded page heap 0.0 MB/cycle.

## Findings

- No JS-side leak: page and worker heaps are flat to the 0.1 MB across
  all 12 cycles, loaded and unloaded. Tile data is fully released on
  unload (worker heap identical every cycle), which was the specific
  concern for the MVT path.
- RSS shows the expected ratchet, not a linear leak: per-cycle unloaded
  RSS growth decays 11.4 -> 6.0 -> 2.5 -> ... -> ~1 MB/cycle,
  approaching a plateau near 500 MB unloaded / ~610 MB loaded on this
  rig. With flat JS heaps that residue is native/malloc retention and
  swiftshader render surfaces (the known ~380+ MB desktop floor), not
  app or maplibre state.
- The ~1 MB/cycle tail at cycle 11 is small and still shrinking; 12
  cycles cannot fully distinguish a slow plateau approach from a tiny
  true native leak. Rerun with more cycles only if a phone session ever
  shows unexplained growth; the portable signal (JS heaps) is clean.
- Coverage caveat: this mode cycles full page load/unload. Within-page
  tile churn (expired-state reloads on query updates, LRU eviction on
  pan) is not exercised; a churn variant that repeatedly triggers
  update_data on a live page is the follow-up if in-session growth is
  ever suspected.
