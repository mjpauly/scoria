-- Add migration script here
CREATE TABLE IF NOT EXISTS location
(
    id          INTEGER PRIMARY KEY NOT NULL CHECK (typeof(id) = 'integer'),
    lat         REAL                NOT NULL CHECK (typeof(lat) = 'real'),
    lon         REAL                NOT NULL CHECK (typeof(lon) = 'real'),
    accuracy    REAL                NOT NULL CHECK (typeof(accuracy) = 'real'),
    speed       REAL                NOT NULL CHECK (typeof(speed) = 'real'),
    course      REAL                NOT NULL CHECK (typeof(course) = 'real'),
    timestamp   INTEGER             NOT NULL CHECK (typeof(timestamp) = 'integer')
);
