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
use stem::database::{init_db, FilteredQuery, LocationRow};
use stem::geojson::DECIMATION_THRESHOLD;

static WORKDIR: &str = "workdir";
static DB_SRC_DIR: &str = "src/app/stem/db";
static DB_FILES: [&str; 3] = ["data.db", "data.db-shm", "data.db-wal"];

#[tokio::main]
async fn main() {
    fs_setup().await;
    // print_db_size();

    // println!("Connecting to database");
    let conn = init_db(db_file(DB_FILES[0])).await.unwrap();
    // print_db_size();

    // println!("Checkpointing database");
    checkpoint_db(&conn).await;
    // print_db_size();

    //asdf
    query_filter_time_compare(&conn).await;
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
) {
    let capped_query = FilteredQuery::new()
        .start(*start_time)
        .end(*end_time)
        .filters(filters.to_owned())
        .decimate(DECIMATION_THRESHOLD);
    println!("full statement:\n{}", capped_query.sql());

    let now = Instant::now();
    let recs = capped_query.fetch_all_with_db(conn).await;
    println!("query time: {:?}", now.elapsed());
    println!("num recs: {}", recs.len());
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
