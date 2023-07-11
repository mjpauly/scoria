-- Adds additional columns to the location data, including altitude, vertical
-- accuracy, speed accuracy, course accuracy, building floor, and source info.
-- Also makes speed and course nullable.

-- All new columns are nullable for backwards compatibility

CREATE TABLE tmplocation
(
    id                          INTEGER NOT NULL PRIMARY KEY,
    timestamp                   INTEGER NOT NULL UNIQUE ON CONFLICT IGNORE,

    latitude                    REAL    NOT NULL,
    longitude                   REAL    NOT NULL,
    horizontal_accuracy         REAL    NOT NULL,

    msl_altitude                REAL,
    ellipsoid_altitude          REAL,
    vertical_accuracy           REAL,
    story                       INTEGER,

    speed                       REAL,
    speed_accuracy              REAL,

    course                      REAL,
    course_accuracy             REAL,

    is_simulated_by_software    INTEGER,
    is_produced_by_accessory    INTEGER
) STRICT;

-- move data from the old table into the new one
INSERT INTO tmplocation (
    id, timestamp,
    latitude, longitude, horizontal_accuracy,
    speed, course
)
SELECT id,timestamp,lat,lon,accuracy,speed,course FROM location;

-- mark unavailable speed and course values as NULL instead of -1
UPDATE tmplocation SET speed  = NULL WHERE speed  < 0;
UPDATE tmplocation SET course = NULL WHERE course < 0;

-- delete old table and rename the new table
DROP TABLE location;
ALTER TABLE tmplocation RENAME TO location;
