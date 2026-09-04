# Grid walk alternatives: Rust-driven, parallel, narrow select

2026-09-02, synthetic dataset: 19,881 points, one per level-8 cell in a
141x141 block near (37.8, -122.5), `grid_bucketed_query` at shift 14
(walk at level 8, d = 0, no filters, no hard_cap), so every occupied
cell is one result row. Desktop macOS (14 cores), sqlx with its bundled
SQLite, warm cache, run twice and stable. Bench tests were temporary
additions to location.rs's tests module; this note is the record.

Absolute walk costs scale with occupied cells; the per-query, per-row,
and per-column overheads below are shape-independent.

## Rust-driven walk vs the recursive CTE

Could the loose index scan move to Rust (seek loop issuing one query
per step, then a batched row fetch), trading the packed-position CTE
for plain code? Measured in the most favorable configuration for Rust
(world rect: no jump logic needed at all):

| | time |
|---|---|
| per-query round-trip floor (`SELECT 1` via sqlx) | 31 us |
| CTE walk, complete (fetch + decode of all rows) | 196 ms |
| Rust loop: walk 679 ms + per-cell newest 683 ms + batch fetch | 1.53 s |

The Rust version issues two queries per cell and 40k x ~34 us is the
whole story: sqlx crosses a channel to the connection's worker thread
per query, while the CTE runs the same B-tree seeks as VDBE opcodes in
one round trip. A synchronous rusqlite loop would cut the per-query
cost to ~1-2 us but adds a second database stack, loses snapshot
consistency across steps, and still loses to the CTE. Fine for
hundreds of cells (~20 ms), fatal for the world-view dense cases this
route exists for.

The packing/CASE tricks are also forced in SQL: the recursive step can
only compute state via scalar subqueries (no LATERAL, no second
reference to the recursive table), and a scalar returns one value, so
both coordinates travel packed. The unpacked per-row variant (one CTE
seed per rectangle row, walking cx only) costs a seek per row of the
rectangle: 262,144 seeds on a level-8 world rect vs O(occupied cells)
for the packed walk, whose 2-D order lets one seek leap any number of
empty rows.

## Parallelizing across connections

SQLite has no intra-query parallelism (one VDBE, one thread), so the
only option is app-side partitioning: split the rectangle into row
bands aligned to query-cell rows, one walk query per band on separate
pool connections (WAL readers don't block each other). Thread
parallelism verified real in the same process (4 spin threads = 1x the
time of 1, 14 cpus).

| configuration | time |
|---|---|
| single walk query | 196 ms |
| 2 / 4 / 8 bands, concurrent | 200 / 310 / 274 ms |
| 4 identical full queries, concurrent | 1.23 s (serial: 784 ms) |
| count(*)-wrapped walk, 1 query | 71 ms |
| count(*)-wrapped walk, 4 concurrent | 105 ms (~2.7x scaling) |

The seek/walk path parallelizes acceptably (count-only rows), but row
materialization contends so badly that concurrent full queries run
slower than serially. Suspected (unverified): SQLite's default
memory-status accounting takes a global mutex on every malloc/free
(`SQLITE_DEFAULT_MEMSTATUS=1`; only `sqlite3_config` before first
connection can disable it, and sqlx never calls it). Amdahl seals it
regardless: the walk is 71 ms of the 196, and hard_cap-sized results
keep production queries row-heavy exactly when the view is big enough
for parallelism to tempt. Phone targets (fewer cores, thermals) only
subtract. If ever revisited: disable memstatus via libsqlite3-sys FFI
at startup, re-run this bench, and only then consider banding.

## Select width

Row materialization dominates the full fetch, so how much is column
count? Same walk returning all 17 location columns, a minimal render
set, and none:

| select | time |
|---|---|
| `l.*` (17 cols, LocationRow) | 193 ms |
| `l.timestamp, l.latitude, l.longitude` | 106 ms |
| `count(*)` (0 rows) | 68 ms |

Of the ~125 ms row cost, ~70% is column extraction/decode (~0.45
us/col/row); fixed per-row overhead is ~2 us. Narrowing the select is
the one real lever found here: ~2x on large bucketed fetches if the
render path can take narrow rows, at the cost of a second fetch (or
wider select) wherever full fields are needed.

## Conclusion

Keep the single recursive-CTE query on a single connection. The CTE's
complexity is load-bearing at exactly the scales that matter; neither
Rust-side walking nor multi-connection parallelism survives contact
with the round-trip and contention numbers. Column narrowing is worth
a look if large-fetch latency becomes a problem.

Update: column narrowing is implemented. The decimated fetches return
NarrowPoint (id, timestamp, lat, lng + two caller-chosen columns CAST
to REAL; location.rs), consumers that need full rows hydrate on the
UNIQUE timestamp (export, the click popup). Measured in dev at ~300k
points retrieved: 5.4 s -> 3.0 s fastbuild, 1.9 s -> 1.0 s under
-c opt.
