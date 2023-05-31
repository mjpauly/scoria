-- Makes the timestamp column have a unique contraint.
-- New data that has a timestamp conflicting with existing data is ignored.

-- Opt for separate PRIMARY KEY, since it auto increments if omitted, which is
-- undesirable behavior for our timestamp, since an auto-incremented value would
-- most certainly be wrong.

-- For future: attempted to order timestamp column after id, but this turned up
-- issues in testing where the first query after migrating might fail due to
-- some strange persistence of the previous schema. To reduce the likelihood of
-- this error, we leave the column order as-is if possible.

CREATE TABLE tmplocation
(
    id          INTEGER NOT NULL PRIMARY KEY,
    timestamp   INTEGER NOT NULL UNIQUE ON CONFLICT IGNORE,
    lat         REAL    NOT NULL,
    lon         REAL    NOT NULL,
    accuracy    REAL    NOT NULL,
    speed       REAL    NOT NULL,
    course      REAL    NOT NULL
) STRICT;

INSERT OR IGNORE INTO tmplocation
SELECT id,timestamp,lat,lon,accuracy,speed,course
FROM location;

DROP TABLE location;

ALTER TABLE tmplocation RENAME TO location;
