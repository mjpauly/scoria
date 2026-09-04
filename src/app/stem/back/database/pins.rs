//! Database interactions for user-defined pin locations

use std::convert::TryFrom;

use common::{pin::Pin, LngLat, ToFront};
use sqlx::{FromRow, Row, SqlitePool};
use tracing::error;

use crate::{
    app_state::set_derived_state, runtime::get_runtime,
    ws_session::send_message_to_front,
};

use super::get_main_db_pool;

/// Save a new pin (if id is None) or update an existing pin.
pub fn save_pin(pin: Pin) {
    get_runtime().spawn(async move {
        let Ok(conn) = get_main_db_pool() else {
            return;
        };
        if let Err(e) = save_pin_to_db(&conn, &pin).await {
            error!("Failed to save pin. {e}");
            return;
        }
        update_derived_pins().await;
    });
}

/// Delete a pin.
pub fn delete_pin(db_idx: i64) {
    get_runtime().spawn(async move {
        let Ok(conn) = get_main_db_pool() else {
            return;
        };
        if let Err(e) = delete_pin_in_db(&conn, db_idx).await {
            error!("Failed to delete pin. {e}");
            return;
        }
        update_derived_pins().await;
    });
}

/// Update the in-memory copy of the pins and send the state to the frontend.
pub async fn update_derived_pins() {
    let Ok(conn) = get_main_db_pool() else {
        return;
    };
    let all_pins = match fetch_all_pins(&conn).await {
        Ok(a) => a,
        Err(e) => {
            error!("Failed to fetch pins. {e}");
            return;
        }
    };
    set_derived_state(|state| state.pins = all_pins);
}

/// Struct representation of a Location row in the table
///
/// Fields that are an array of values like `lists`, `tags`, and `boundary` are
/// encoded as json strings. `boundary` is the only nullable field.
#[derive(Clone, FromRow, Debug)]
struct PinRow {
    id: i64,
    lng: f64, // location of the icon / name
    lat: f64,
    name: String,
    icon: String,             // emoji icon
    lists: String,            // json array of strings
    tags: String,             // json array of (string, string) key values
    boundary: Option<String>, // optional json array of (lng, lat) (unused)
}

impl TryFrom<PinRow> for Pin {
    type Error = serde_json::Error;

    fn try_from(pr: PinRow) -> Result<Self, Self::Error> {
        Ok(Pin {
            id: Some(pr.id),
            lnglat: LngLat {
                lng: pr.lng,
                lat: pr.lat,
            },
            name: pr.name,
            icon: pr.icon,
            lists: serde_json::from_str(&pr.lists)?,
            tags: serde_json::from_str(&pr.tags)?,
            boundary: pr
                .boundary
                .map(|b| serde_json::from_str(&b))
                .transpose()?, // Option<Result<B, E>> to Result<Option<B>, E>
        })
    }
}

pub async fn fetch_all_pins(conn: &SqlitePool) -> anyhow::Result<Vec<Pin>> {
    Ok(sqlx::query_as::<_, PinRow>("SELECT * FROM pins")
        .fetch_all(conn)
        .await?
        .into_iter()
        .map(|p| p.try_into())
        .collect::<Result<Vec<Pin>, <Pin as TryFrom<PinRow>>::Error>>()?)
}

pub async fn delete_pin_in_db(
    conn: &SqlitePool,
    id: i64,
) -> anyhow::Result<()> {
    sqlx::query("DELETE FROM pins WHERE id = ?")
        .bind(id)
        .execute(conn)
        .await?;
    Ok(())
}

pub async fn save_pin_to_db(
    conn: &SqlitePool,
    pin: &Pin,
) -> anyhow::Result<()> {
    let lists_str = serde_json::to_string(&pin.lists)?;
    let tags_str = serde_json::to_string(&pin.tags)?;
    let boundary_str_option = pin
        .boundary
        .as_ref()
        .map(serde_json::to_string)
        .transpose()?;
    if let Some(id) = pin.id {
        sqlx::query(
            "UPDATE pins
            SET lng = ?,
                lat = ?,
                name = ?,
                icon = ?,
                lists = ?,
                tags = ?,
                boundary = ?
            WHERE id = ?
            RETURNING id",
        )
        .bind(pin.lnglat.lng)
        .bind(pin.lnglat.lat)
        .bind(&pin.name)
        .bind(&pin.icon)
        .bind(lists_str)
        .bind(tags_str)
        .bind(boundary_str_option)
        .bind(id)
        .execute(conn)
        .await?;
    } else {
        // If we don't wrap this `fetch_one` in a transaction, then the newly
        // added row is not always returned on the subsequent call to get all
        // pins (in testing this was nondeterministic, failing to return the new
        // row most of the time). This is probably because Sqlite's implicit
        // transactions are not always committed if there is another statement
        // immediately after. This was not an issue for a simple `execute`.
        let mut tx = conn.begin().await?;
        let new_id = sqlx::query(
            "INSERT INTO pins (
                lng, lat,
                name, icon,
                lists, tags,
                boundary
            )
            VALUES (?,?,?,?,?,?,?)
            RETURNING id",
        )
        .bind(pin.lnglat.lng)
        .bind(pin.lnglat.lat)
        .bind(&pin.name)
        .bind(&pin.icon)
        .bind(lists_str)
        .bind(tags_str)
        .bind(boundary_str_option)
        .fetch_one(&mut *tx)
        .await?
        .get(0);
        tx.commit().await?;
        send_message_to_front(ToFront::NewPinId(new_id));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use crate::local::test_setup;

    use super::*;

    fn get_test_pins() -> Vec<Pin> {
        vec![
            Pin {
                lnglat: LngLat {
                    lng: -122.50,
                    lat: 37.80,
                },
                name: "ALPACA".into(),
                icon: "😺".into(),
                ..Default::default()
            },
            Pin {
                lnglat: LngLat {
                    lng: -122.51,
                    lat: 37.81,
                },
                name: "Charlie".into(),
                icon: "🍨".into(),
                ..Default::default()
            },
            Pin {
                lnglat: LngLat {
                    lng: -122.51,
                    lat: 37.80,
                },
                lists: vec![
                    "Favs".into(),
                    "Food".into(),
                    "Long list name".into(),
                ],
                tags: vec![("Hours".into(), "9-5".into())],
                ..Default::default()
            },
            Pin {
                lnglat: LngLat {
                    lng: -122.47,
                    lat: 37.77,
                },
                name: "Delta".into(),
                icon: "⬛️".into(),
                ..Default::default()
            },
            Pin {
                lnglat: LngLat {
                    lng: -122.40,
                    lat: 37.70,
                },
                name: "Bravo".into(),
                ..Default::default()
            },
        ]
    }

    async fn add_test_pins(conn: &SqlitePool) {
        for p in get_test_pins().iter() {
            save_pin_to_db(conn, p).await.unwrap();
        }
    }

    #[tokio::test]
    async fn test_simple_pins_db() {
        test_setup("test_simple_pins_db/").await;
        let conn = get_main_db_pool().unwrap();
        add_test_pins(&conn).await;

        let pins = fetch_all_pins(&conn).await.unwrap();

        let mut expected = get_test_pins();
        for (i, p) in expected.iter_mut().enumerate() {
            p.id = Some(i as i64 + 1);
        }
        assert_eq!(expected, pins);
    }

    #[tokio::test]
    async fn test_delete_pins() {
        test_setup("test_delete_pins/").await;
        let conn = get_main_db_pool().unwrap();
        add_test_pins(&conn).await;
        delete_pin_in_db(&conn, 1).await.unwrap();

        let pins = fetch_all_pins(&conn).await.unwrap();

        let mut expected = get_test_pins();
        expected.remove(0);
        for (i, p) in expected.iter_mut().enumerate() {
            p.id = Some(i as i64 + 2);
        }
        assert_eq!(expected, pins);
    }
}
