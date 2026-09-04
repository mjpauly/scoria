//! Logic for mounted databases in addition to the main database.
//!
//! Currently can only view location tracks, not pins, in mounted dbs.
//!
//! Derived state stores mounted ids, which can be read by frontend.
//! If a mounted id exists in derived state which is not in frontend state, a
//! new frontend record is created which user can name and enable/disable.
//!
//! If a id exists in frontend state that is not in derived state, then indicate
//! it's not found on disk and have option to delete the frontend record
//!
//! When deleting, send msg to backend say to delete, which corresponds to a
//! return message saying db deleted, where frontend record gets deleted.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::{fs, os::unix::prelude::PermissionsExt};
use tracing::error;

use common::mounted::{MountID, MAIN_DB_MOUNT_ID};
use common::state::DbStatus;

use crate::app_state::{get_front_state, set_derived_state, AppState};
use crate::logs::LogErrorAndContinue;
use crate::paths::{get_mount_root_dir, DB_FNAME};
use crate::tz::get_system_tz;
use crate::ws_session;

use super::{get_db_for_id, open_db_with_status};

/// Open connections to all databases currently mounted on disk, excluding the
/// main database. This is run on foregrounding, since the mounted databases are
/// just for viewing, not logging new data.
pub async fn open_all_mounted() {
    try_open_all_mounted()
        .await
        .context("init mounted dbs")
        .log_error_and_continue();
}

/// Fallible routine for opening connections to each database. Each open
/// runs in the background (migrations can take seconds), reporting its
/// status in derived state and inserting its pool when ready.
async fn try_open_all_mounted() -> Result<()> {
    let mount_root_dir = get_mount_root_dir();
    // might already exist, skip inspecting error
    let _ = fs::create_dir(&mount_root_dir);
    let entries =
        fs::read_dir(&mount_root_dir).context("reading mount root dir")?;
    let mut ids = Vec::new();
    for entry in entries {
        let entry = match entry.context("reading dir entry") {
            Ok(entry) => entry,
            Err(e) => {
                error!("{e:?}");
                continue;
            }
        };
        let db_id = entry.file_name();
        let db_id_str = db_id.to_string_lossy();
        let mount_dir = mount_root_dir.join(&db_id);
        let Ok(id) = db_id_str
            .parse::<MountID>()
            .context("parsing mount dir name as id")
            .map_err(|e| error!("{e:?}"))
        else {
            // can't parse as id -> remove unknown directory
            remove_mount_dir(&mount_dir);
            continue;
        };
        ids.push(id);
    }
    // drop mounts that are no longer on disk (keeps the main database)
    AppState::global()
        .dbs
        .lock()
        .unwrap()
        .retain(|id, _| *id == MAIN_DB_MOUNT_ID || ids.contains(id));
    set_derived_state(|s| {
        s.mounted_dbs_on_disk
            .retain(|id, _| *id == MAIN_DB_MOUNT_ID || ids.contains(id));
    });
    for id in ids {
        // already open from a previous foregrounding
        if get_db_for_id(id).is_some() {
            continue;
        }
        spawn_open_mounted(id, mount_root_dir.join(id.to_string()));
    }
    Ok(())
}

/// Open the database in `mount_dir` in the background, tracking its status
/// in derived state under `id` and inserting its pool once ready.
fn spawn_open_mounted(id: MountID, mount_dir: PathBuf) {
    set_derived_state(|s| {
        s.mounted_dbs_on_disk.insert(
            id,
            DbStatus::Opening {
                stage: "starting".into(),
                progress: None,
            },
        )
    });
    tokio::spawn(async move {
        let db_path = mount_dir.join(DB_FNAME);
        let result =
            open_db_with_status(db_path.to_string_lossy().to_string(), id)
                .await
                .context("error opening mounted database");
        let status = match result {
            Ok(conn) => {
                AppState::global().dbs.lock().unwrap().insert(id, conn);
                DbStatus::Ready
            }
            Err(e) => {
                error!("{e:?}");
                DbStatus::Error(format!("{e:?}"))
            }
        };
        set_derived_state(|s| s.mounted_dbs_on_disk.insert(id, status));
        if AppState::global().ws_addr.lock().unwrap().is_some() {
            crate::core::update_on_foregrounding();
        }
    });
}

/// On backgrounding, close connections to all mounted databases.
pub async fn close_all_mounted() {
    if let Err(e) = try_close_all_mounted().await.context("close mounted dbs") {
        error!("{e:?}");
    }
}

/// Close all mounted database connections (excludes main database). Derived
/// state is only updated on foregrounding.
async fn try_close_all_mounted() -> Result<()> {
    let mut connections_to_close = Vec::new();
    {
        let app_state = AppState::global();
        let mut connections = app_state.dbs.lock().unwrap();
        let ids_to_remove = connections
            .keys()
            .filter(|id| **id != MAIN_DB_MOUNT_ID)
            .cloned()
            .collect::<Vec<_>>();
        for id in ids_to_remove.iter() {
            connections_to_close.push(connections.remove(id));
        }
    }
    for conn in connections_to_close.into_iter().flatten() {
        conn.close().await;
    }
    Ok(())
}

/// Remove a mount directory, without any special error handling other than
/// logging. Used to cancel an already failing mount process.
fn remove_mount_dir(mount_dir: &Path) {
    fs::remove_dir_all(mount_dir)
        .context("removing mount dir")
        .log_error_and_continue();
}

/// Mount a database from a path.
pub async fn mount_db(path: PathBuf) {
    let (new_id, new_mount_dir) =
        match get_new_mount_dir().context("creating new db mount dir") {
            Ok(x) => x,
            Err(e) => {
                error!("{e:?}");
                ws_session::send_error_popup("failed to mount database");
                return;
            }
        };
    let new_path = new_mount_dir.join(DB_FNAME);
    if let Err(e) = fs::copy(&path, &new_path)
        .context("copying file")
        .and_then(|_| set_writable(&new_path).context("making file writable"))
    {
        error!("mounting db: {e:?}");
        remove_mount_dir(&new_mount_dir);
        ws_session::send_error_popup("failed to mount database");
        return;
    }
    let fname = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| {
            // can't get filename -> use current timestamp
            let tz = get_system_tz().unwrap_or(jiff::tz::TimeZone::UTC);
            let now = jiff::Timestamp::now();
            let zdt = now.to_zoned(tz);
            zdt.to_string()
        });
    set_derived_state(|s| {
        s.last_mounted = Some((new_id, fname));
    });
    spawn_open_mounted(new_id, new_mount_dir);
}

/// fs::copy copies permission bits, so we need to make sure the file is
/// writable.
fn set_writable(path: &PathBuf) -> Result<()> {
    let f = fs::File::open(path)?;
    let metadata = f.metadata()?;
    let mut permissions = metadata.permissions();
    permissions.set_mode(0o640);
    Ok(())
}

fn get_new_mount_dir() -> Result<(MountID, PathBuf)> {
    let mount_root_dir = get_mount_root_dir();
    fs::create_dir_all(&mount_root_dir)?;
    let entries =
        fs::read_dir(&mount_root_dir).context("reading mount root dir")?;
    // choose new id to be greater than the current largest to preserve id
    // ordering by import time
    let mut max_id = MAIN_DB_MOUNT_ID;
    for entry in entries {
        let id = entry
            .context("reading dir entry")?
            .file_name()
            .to_string_lossy()
            .parse::<MountID>()
            .context("parsing dir name as mount id")?;
        if id > max_id {
            max_id = id;
        }
    }
    let new_id = max_id + 1;
    let new_dir = mount_root_dir.join(new_id.to_string());
    fs::create_dir(&new_dir).context("creating new dir")?;
    Ok((new_id, new_dir))
}

pub async fn delete_mounted_db(id: MountID) {
    debug_assert!(id != MAIN_DB_MOUNT_ID); // should not be possible in ui
    let maybe_conn = {
        let state = AppState::global();
        let mut mounted = state.dbs.lock().unwrap();
        mounted.remove(&id)
    };
    set_derived_state(|s| s.mounted_dbs_on_disk.remove(&id));
    if let Some(conn) = maybe_conn {
        conn.close().await;
    }
    // evict the mount's derived map data
    crate::map::mvt::remove_tileset(id).await;
    let mount_root_dir = get_mount_root_dir();
    let mount_dir = mount_root_dir.join(id.to_string());
    if let Err(e) = fs::remove_dir_all(mount_dir).context("removing mount dir")
    {
        error!("{e:?}");
        ws_session::send_error_popup("error deleting database");
    } else {
        ws_session::send_success_popup("Database deleted");
    }
}

/// Return the name for a database id if assigned, otherwise just the id. Used
/// for helpful error messages.
pub fn db_debug_name(id: &MountID) -> String {
    get_front_state(|s| s.mounted_db_settings.get(id).cloned())
        .flatten()
        .map(|setting| format!("{} ({})", setting.name, id))
        .unwrap_or_else(|| id.to_string())
}

#[cfg(test)]
pub mod tests {
    use crate::app_state::{get_derived_state, AppState};
    use crate::database::mounted::open_all_mounted;
    use crate::database::{open_db, wait_for_dbs};
    use crate::init;
    use crate::local::local_fs_setup;
    use common::state::DbStatus;

    #[tokio::test]
    async fn valid_db_on_disk_opens_without_error() {
        let dir = "valid_db_on_disk_opens_without_error/";
        let mount_id = 99;
        let paths = local_fs_setup(dir);
        let mount_dir =
            paths.documents_dir.join("mount").join(mount_id.to_string());
        std::fs::create_dir_all(&mount_dir).unwrap();
        let db_path = mount_dir.join("data.db").to_string_lossy().to_string();
        open_db(db_path).await.unwrap().close().await;
        init(paths, "1.test.0".into()).await;
        open_all_mounted().await;
        wait_for_dbs().await;
        // should have a valid connection
        assert!(AppState::global()
            .dbs
            .lock()
            .unwrap()
            .get(&mount_id)
            .is_some());
        // and no errors
        assert!(get_derived_state(|s| s
            .mounted_dbs_on_disk
            .get(&mount_id)
            .unwrap()
            .is_ready()));
    }

    #[tokio::test]
    async fn invalid_db_on_disk_opens_with_error() {
        let dir = "invalid_db_on_disk_opens_with_error/";
        let mount_id = 99;
        let paths = local_fs_setup(dir);
        let mount_dir =
            paths.documents_dir.join("mount").join(mount_id.to_string());
        std::fs::create_dir_all(&mount_dir).unwrap();
        let db_path = mount_dir.join("data.db");

        // create a text file and not a database
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .open(&db_path)
            .unwrap();
        std::io::Write::write_all(&mut file, b"asdf").unwrap();

        init(paths, "1.test.0".into()).await;
        open_all_mounted().await;
        wait_for_dbs().await;
        // no valid connection
        assert!(AppState::global()
            .dbs
            .lock()
            .unwrap()
            .get(&mount_id)
            .is_none());
        // and there an errors
        assert!(get_derived_state(|s| matches!(
            s.mounted_dbs_on_disk.get(&mount_id).unwrap(),
            DbStatus::Error(_)
        )));
    }
}
