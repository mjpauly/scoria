-- Level indexes over the grid cells of the previous migration, each ~16% of
-- the table. database::open_db runs the gx/gy backfill (database/grid.rs)
-- between the previous migration and this one, so the indexes are bulk
-- sorted builds rather than per-row inserts, and no query sees a NULL cell.
-- The longitude index is superseded: the grid serves spatial queries and the
-- timestamp autoindex serves time ranges
-- (doc/decimation/probe-results/time-range-bench.md).
CREATE INDEX IF NOT EXISTS location_cell8 ON location(cy8, cx8, timestamp);
CREATE INDEX IF NOT EXISTS location_cell10 ON location(cy10, cx10, timestamp);
CREATE INDEX IF NOT EXISTS location_cell12 ON location(cy12, cx12, timestamp);
DROP INDEX IF EXISTS lngtimeindex;
