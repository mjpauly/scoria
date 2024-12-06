//! Test database queries. Not really a `test` per se, but a development tool
//! for examining the performance of different kinds of queries and looking at
//! the contents of the database more closely.

use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::time::Instant;

use sqlx::{
    sqlite::SqliteRow, Column, FromRow, Row, SqlitePool, Value, ValueRef,
};
use time::macros::datetime;

use common::filters::{DataStream, Filter, FilterOp};
use stem::database::{open_db, FilteredQuery, LocationRow};
use stem::map::geojson::DECIMATION_THRESHOLD;

static WORKDIR: &str = "workdir";
static DB_SRC_DIR: &str = "src/app/stem/db";
static DB_FILES: [&str; 3] = ["data.db", "data.db-shm", "data.db-wal"];

#[tokio::main]
async fn main() {
    fs_setup().await;
    // print_db_size();

    // println!("Connecting to database");
    let conn = open_db(db_file(DB_FILES[0])).await.unwrap();
    // print_db_size();

    // println!("Checkpointing database");
    checkpoint_db(&conn).await;
    // print_db_size();

    //asdf
    // query_filter_time_compare(&conn).await;
    index_time_compare(&conn).await;
}

async fn fs_setup() {
    println!("Run dir: {}", std::env::current_dir().unwrap().display());

    // clear out prior contents if they exist and recreate the directory
    if fs::metadata(WORKDIR).is_ok() {
        fs::remove_dir_all(WORKDIR).unwrap();
    }
    fs::create_dir_all(WORKDIR).unwrap();

    // copy over the database
    for file in DB_FILES {
        let src = src_db_file(file);
        let dst = db_file(file);
        fs::copy(src, dst).unwrap();
    }
}

fn src_db_file(file: &str) -> String {
    format!("{DB_SRC_DIR}/{file}")
}

fn db_file(file: &str) -> String {
    format!("{WORKDIR}/{file}")
}

/// Print out the size of the data base files
#[allow(dead_code)]
fn print_db_size() {
    println!();
    for file in DB_FILES {
        let len = fs::metadata(db_file(file))
            .map(|meta| meta.len() as f64 / 1e6)
            .unwrap_or(0.);
        println!("{file} is {len} MB large");
    }
    println!();
}

pub async fn checkpoint_db(conn: &SqlitePool) {
    sqlx::query("VACUUM; PRAGMA wal_checkpoint(FULL);")
        .execute(conn)
        .await
        .unwrap();
}

pub async fn query_filter_time_compare(conn: &SqlitePool) {
    let start_time = datetime!(2023-01-01 0:00 UTC);
    // let start_time = datetime!(2023-10-28 0:00 UTC);
    // let start_time = datetime!(2023-09-28 0:00 UTC);
    let end_time = datetime!(2033-11-04 0:00 UTC);
    // let start_time = datetime!(2023-09-28 0:00 UTC);
    // let end_time = datetime!(2023-09-28 2:35 UTC);
    let filters = get_filters();

    no_filtering(conn, &start_time, &end_time).await;
    println!();
    filter_during_query(conn, &start_time, &end_time, &filters).await;
    // can't compare the results since we're decimating on the id of the row,
    // which isn't aligned to the start/end times exactly.

    // dump_id_timestamp_cols(conn).await;
}

pub async fn no_filtering(
    conn: &SqlitePool,
    start_time: &time::OffsetDateTime,
    end_time: &time::OffsetDateTime,
) {
    let now = Instant::now();
    let recs = sqlx::query_as::<_, LocationRow>(
        "SELECT * FROM location
        WHERE timestamp >= ? AND timestamp < ?
        ORDER BY timestamp ASC",
    )
    .bind(start_time.unix_timestamp())
    .bind(end_time.unix_timestamp())
    .fetch_all(conn)
    .await
    .unwrap();
    println!("query time: {:?}", now.elapsed());
    println!("num recs: {}", recs.len());

    // filtering takes little time, so we just compare the time it takes to
    // retrieve more records in the first place, as opposed to getting only
    // those that we want when doing our database query
}

fn get_filters() -> Vec<Filter> {
    vec![
        Filter {
            id: 0,
            enabled: true,
            datastream: DataStream::HorizAccuracy,
            op: FilterOp::GreaterThan,
            threshold: 20.0,
        },
        Filter {
            id: 1,
            enabled: true,
            datastream: DataStream::Speed,
            op: FilterOp::GreaterThan,
            threshold: 10.0,
        },
    ]
}

pub async fn filter_during_query(
    conn: &SqlitePool,
    start_time: &time::OffsetDateTime,
    end_time: &time::OffsetDateTime,
    filters: &[Filter],
) -> std::time::Duration {
    let now = Instant::now();
    let recs = FilteredQuery::builder()
        .start(
            jiff::Timestamp::from_nanosecond(start_time.unix_timestamp_nanos())
                .unwrap(),
        )
        .end(
            jiff::Timestamp::from_nanosecond(end_time.unix_timestamp_nanos())
                .unwrap(),
        )
        .filters(filters.to_owned())
        .limit(DECIMATION_THRESHOLD)
        .build()
        .fetch_decimated_with_db(conn)
        .await;
    let elapsed = now.elapsed();
    println!("query time: {:?}", now.elapsed());
    println!("num recs: {}", recs.len());
    elapsed
}

/// Findings: even with different time filters, there's no real speedup when
/// creating an index on timestamp. Perhaps this is because it's a UNIQUE
/// column, which is enforced by already having an index on it.
///
/// Adding an extra index on longitude or latitude speeds up queries where the
/// index significantly cuts down on the number of records to return, but also
/// slows queries where the indes does not cut down the number of records much.
///
/// In this testing, the best performance occurs when creating an index on
/// (location,latitude,timestamp). However, testing this in the actual app
/// environment there doesn't seem to be much speedup in view-bounded queries,
/// that would most take advantage of the index on longitudes.
///
/// Seems like this is because having the filter on horizontal accuracy
/// nullifies all the speedup.
///
/// # Testing with primary key
///
/// ## PRIMARY KEY (timestamp,longitude,latitude,horizontal_accuracy)
///
/// speedup: 19.429% MH
/// speedup: 19.698%
/// speedup: 14.079%
/// speedup: 12.000%
/// speedup: 15.756% Monterey
/// speedup: 19.331%
/// speedup: 14.139%
/// speedup: -17.197%
/// speedup: 11.666% Bay
/// speedup: 4.847%
/// speedup: -0.631%
/// speedup: -9.630%
/// speedup: 10.702% TJ
/// speedup: 6.130%
/// speedup: 4.530%
/// speedup: -9.231%
///
/// ## PRIMARY KEY (longitude,latitude,timestamp,horizontal_accuracy)
///
/// speedup: 48.263% MH         wide time
/// speedup: 47.645%            first half time
/// speedup: 45.029%            last month time
/// speedup: -6.573%            no time
/// speedup: 40.558% Monterey
/// speedup: 46.199%
/// speedup: 40.026%
/// speedup: -476.190%
/// speedup: 18.892% Bay
/// speedup: -23.667%
/// speedup: -19.427%
/// speedup: -20115.823%
/// speedup: 19.285% TJ
/// speedup: -14.468%
/// speedup: -25.813%
/// speedup: -25438.931%
///
/// ## INDEX ON (timestamp,longitude) (BEST)
///
/// speedup: 27.914%
/// speedup: 25.810%
/// speedup: 27.274%
/// speedup: -7.065%
/// speedup: 18.674%
/// speedup: 28.908%
/// speedup: 28.936%
/// speedup: 15.504%
/// speedup: -0.141%
/// speedup: -5.208%
/// speedup: -2.468%
/// speedup: -11.765%
/// speedup: -0.070%
/// speedup: -3.445%
/// speedup: -1.785%
/// speedup: 56.508%
/// Average: 12.348897044520792
///
/// ## Averages
///
/// - INDEX (timestamp,longitude): 7-15% (BEST)
/// - INDEX (timestamp,latitude): 5%
/// - INDEX (timestamp,longitude,latitude,horizontal_accuracy): 0.5-15%
/// - INDEX (longitude,latitude,timestamp,horizontal_accuracy): -3000%
///
/// - KEY (timestamp,longitude): 7-13%
/// - KEY (timestamp,latitude): 5%
/// - KEY (timestamp,longitude,latitude): -3-3%
/// - KEY (timestamp,longitude,horizontal_accuracy): -5-4%
/// - KEY (timestamp,longitude,latitude,horizontal_accuracy): 3-10%
///
/// ## Conclusions
///
/// Having primary keys on 4 cols grows database size from 39 to 55 MB. This is
/// probably because the primary key itself is implemented as an index, so there
/// isn't actually much benefit to doing this instead of creating a new index.
///
/// Filtering by timestamp then space does improve most queries somewhat, and
/// incurs a small cost where the index can't help cut down on the number of
/// records.
///
/// Filtering by space then timestamp helps speed up space queries by up to 50%,
/// but can incur enormous cost on time-limited queries. Not recommended.
///
/// Indexing on (timestamp,longitude) in that order is the best. DB grows from
/// 39 to 49 MB regardless of whether it's just a new index or a new primary
/// key, but the performance is slightly higher with a new index on average.
///
/// Since the speedup is not more significant (at most 30% on the best index
/// setting of (timestamp,longitude)), we will punt this question of speeding up
/// queries until later, when it matters more.
///
/// Having this extra filtering based on space helps when looking at data across
/// all times, since this is when time filtering doesn't help much.
async fn index_time_compare(conn: &SqlitePool) {
    let filters_set = [
        bound_filters(-121.642719, 37.11844, -121.62561, 37.14312), // MH
        bound_filters(-121.924, 36.577, -121.827, 36.639),          // monterey
        bound_filters(-125., 30., -110., 40.),                      // bay
        // bound_filters(-122.163635, 37.432806, -122.151907, 38.441730), // TJ
        bound_filters(-125., 37.432806, -110., 38.441730), // TJ lat slice
    ];
    let start_times = [
        datetime!(2023-01-01 0:00 UTC),
        datetime!(2023-01-01 0:00 UTC),
        datetime!(2023-10-01 0:00 UTC),
        datetime!(2023-10-01 0:00 UTC),
    ];
    let end_times = [
        datetime!(2033-01-01 0:00 UTC),
        datetime!(2023-06-01 0:00 UTC),
        datetime!(2033-01-01 0:00 UTC),
        datetime!(2023-06-01 0:00 UTC),
    ];
    let mut slower = vec![];
    for filters in &filters_set {
        for i in 0..4 {
            let dur = filter_during_query(
                conn,
                &start_times[i],
                &end_times[i],
                filters,
            )
            .await;
            slower.push(dur);
        }
    }
    println!();
    print_db_size();
    // sqlx::query("CREATE INDEX lo ON location(timestamp,longitude,latitude);")
    // sqlx::query("CREATE INDEX lo ON location(timestamp,longitude,latitude,horizontal_accuracy);")
    sqlx::query("CREATE INDEX asdf ON location(timestamp,longitude);") // BEST
        // sqlx::query("CREATE INDEX lo ON location(timestamp,latitude);")
        /*
        sqlx::query(
                    "
            CREATE TABLE tmplocation
            (
                id                          INTEGER NOT NULL,
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
                is_produced_by_accessory    INTEGER,
                was_imported                INTEGER NOT NULL DEFAULT 0,
                PRIMARY KEY (timestamp,longitude)
            ) STRICT;
            -- move data from the old table into the new one
            INSERT INTO tmplocation (
                id, timestamp,
                latitude, longitude, horizontal_accuracy,
                msl_altitude, ellipsoid_altitude, vertical_accuracy, story,
                speed, speed_accuracy, course, course_accuracy,
                is_simulated_by_software, is_produced_by_accessory,
                was_imported
            )
            SELECT
                id, timestamp,
                latitude, longitude, horizontal_accuracy,
                msl_altitude, ellipsoid_altitude, vertical_accuracy, story,
                speed, speed_accuracy, course, course_accuracy,
                is_simulated_by_software, is_produced_by_accessory,
                was_imported
            FROM location;
            DROP TABLE location;
            ALTER TABLE tmplocation RENAME TO location;
            VACUUM; PRAGMA wal_checkpoint(FULL);
                            ",
                )
                */
        .execute(conn)
        .await
        .unwrap();
    sqlx::query("PRAGMA wal_checkpoint(TRUNCATE);")
        .execute(conn)
        .await
        .unwrap();
    print_db_size();
    // for filters in &filters_set {
    // filter_during_query(conn, &start_time, &end_time, &filters).await;
    // }
    let mut speedier = vec![];
    for filters in &filters_set {
        for i in 0..4 {
            let dur = filter_during_query(
                conn,
                &start_times[i],
                &end_times[i],
                filters,
            )
            .await;
            speedier.push(dur);
        }
    }
    let mut sum = 0.0;
    for i in 0..speedier.len() {
        let speedup = (slower[i].as_micros() as f64
            - speedier[i].as_micros() as f64)
            / slower[i].as_micros() as f64;
        sum += speedup * 100.0;
        println!("speedup: {:.3}%", speedup * 100.);
    }
    println!("Average: {}", sum / speedier.len() as f64);
    println!();
}

fn bound_filters(
    sw_lng: f64,
    sw_lat: f64,
    ne_lng: f64,
    ne_lat: f64,
) -> Vec<Filter> {
    vec![
        Filter {
            id: 0,
            enabled: true,
            datastream: DataStream::Lon,
            op: FilterOp::LessThan,
            threshold: sw_lng,
        },
        Filter {
            id: 1,
            enabled: true,
            datastream: DataStream::Lat,
            op: FilterOp::LessThan,
            threshold: sw_lat,
        },
        Filter {
            id: 2,
            enabled: true,
            datastream: DataStream::Lon,
            op: FilterOp::GreaterThan,
            threshold: ne_lng,
        },
        Filter {
            id: 3,
            enabled: true,
            datastream: DataStream::Lat,
            op: FilterOp::GreaterThan,
            threshold: ne_lat,
        },
        Filter {
            id: 0,
            enabled: true,
            datastream: DataStream::HorizAccuracy,
            op: FilterOp::GreaterThan,
            threshold: 100.0,
        },
    ]
}

#[derive(Clone, FromRow, Debug)]
struct IdTimestamp {
    pub id: i64,
    pub timestamp: i64,
}

/// Dump the id and timestamp columns to file so we can plot/examine the
/// monotonicity of the id column to see if it's good enough to use for
/// decimating. Result: yes, it is good enough. Only 0.1% of rows do not
/// increment by 1.
#[allow(dead_code)]
async fn dump_id_timestamp_cols(conn: &SqlitePool) {
    let recs =
        sqlx::query_as::<_, IdTimestamp>("SELECT id,timestamp FROM location")
            .fetch_all(conn)
            .await
            .unwrap();
    let mut f =
        fs::File::create(format!("{WORKDIR}/id_timestamp.csv")).unwrap();
    writeln!(f, "id,timestamp").unwrap();
    for rec in recs {
        writeln!(f, "{},{}", rec.id, rec.timestamp).unwrap();
    }
    println!("Completed dump");
}

/// Convert a database query result to a hashmap of String->String.
/// Requires editing for the correct type (TODO: make decode type generic).
///
/// let query = query_builder.build();
/// let row = query.fetch_one(conn).await.unwrap();
/// dbg!(row_to_json(row));
#[allow(dead_code)]
fn row_to_json(row: SqliteRow) -> HashMap<String, String> {
    let mut result = HashMap::new();
    for col in row.columns() {
        let value = row.try_get_raw(col.ordinal()).unwrap();
        let value = match value.is_null() {
            true => "NULL".to_string(),
            false => value.to_owned().decode::<i64>().to_string(),
        };
        result.insert(col.name().to_string(), value);
    }

    result
}
