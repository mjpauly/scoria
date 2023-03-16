-- Add migration script here
-- STRICT requires all columns to specify a datatype and that inserted data be
-- of that datatype. This mitigates some of the problems with SQLite's lax
-- typing.
CREATE TABLE IF NOT EXISTS location
(
    id          INTEGER PRIMARY KEY NOT NULL,
    lat         REAL                NOT NULL,
    lon         REAL                NOT NULL,
    accuracy    REAL                NOT NULL,
    speed       REAL                NOT NULL,
    course      REAL                NOT NULL,
    timestamp   INTEGER             NOT NULL
) STRICT;
