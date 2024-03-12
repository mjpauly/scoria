//! Passes or doesn't pass tile requests to Maptiler or the frontend, depending
//! on if the tile is needed or not, and whether it's already cached on the file
//! system. Tiles that would fall entirely within the unexplored part of the map
//! should not be passed to the frontend to reduce requests and flickering.
//! Since the unexplored area can change rapidly, these responses have a max-age
//! of 2 seconds.
//!
//! # Implementation notes
//!
//! All resources are stored on disk unmodified. When retrieving json files to
//! serve to the frontend, maptiler urls are replaced with the url of the
//! backend so requests are proxied through it.
//!
//! If there's a cache hit, the modified time of the file is checked to see if
//! it's more than a week old. If it is, it attempts to refresh the file and
//! serve that. But if that fails, it continues serving the cached file.
//!
//! # URLs to handle
//!
//! Style url:
//! - https://api.maptiler.com/maps/basic-v2/style.json?key=
//!
//! Tile JSON url:
//! - styles: basic-v2, dataviz, streets-v2, topo-v2, outdoor-v2, hybrid
//! - https://api.maptiler.com/tiles/v3/tiles.json?key=
//! Tile url (inside tile JSON):
//! - https://api.maptiler.com/tiles/v3/{z}/{x}/{y}.pbf?key=
//!
//! Glyphs url:
//! - styles: basic-v2, dataviz, streets-v2, topo-v2, outdoor-v2, hybrid
//! - https://api.maptiler.com/fonts/{fontstack}/{range}.pbf?key=
//!
//! Contours JSON url:
//! - styles: topo-v2, outdoor-v2
//! - https://api.maptiler.com/tiles/contours/tiles.json?key=
//! Countours tile url:
//! - https://api.maptiler.com/tiles/contours/{z}/{x}/{y}.pbf?key=
//!
//! Terrain RGB JSON url:
//! - styles: topo-v2, outdoor-v2
//! - https://api.maptiler.com/tiles/terrain-rgb-v2/tiles.json?key=
//! Terrain RBG tile url:
//! - https://api.maptiler.com/tiles/terrain-rgb-v2/{z}/{x}/{y}.webp?key=
//!
//! Outdoor ("outdoor")
//! - https://api.maptiler.com/tiles/outdoor/tiles.json?key=
//! - https://api.maptiler.com/tiles/outdoor/{z}/{x}/{y}.pbf?key=
//!
//! Satellite ("satellite")
//! - https://api.maptiler.com/tiles/satellite-v2/tiles.json?key=
//! - https://api.maptiler.com/tiles/satellite-v2/{z}/{x}/{y}.jpg?key=
//!
//!
//! Sprite urls:
//!
//! - https://api.maptiler.com/maps/topo-v2/sprite
//! - https://api.maptiler.com/maps/outdoor-v2/sprite
//! - https://api.maptiler.com/maps/streets-v2-dark/sprite
//! - https://api.maptiler.com/maps/topo-v2-dark/sprite
//! - https://api.maptiler.com/maps/outdoor-v2-dark/sprite

use std::cmp::Ordering;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::SystemTime;

use actix_web::http::header::{
    CacheDirective, ACCESS_CONTROL_ALLOW_ORIGIN, CONTENT_TYPE,
};
use actix_web::HttpRequest;
use actix_web::{routes, web, HttpResponse, Responder};
use obfstr::obfstr;
use tokio::sync::OnceCell;
use tracing::{debug, error, info, trace};
use walkdir::WalkDir;

use super::automap::{automap_is_on, tile_has_been_visited};
use super::coords::TileXYZ;
use crate::app_state::AppState;
use crate::paths::get_map_cache_dir;
use crate::server::no_caching_directives;

// references to the remote url are replaced with the backend url when serving
// tile jsons
const MAPTILER_URL: &str = "https://api.maptiler.com";

#[cfg(not(feature = "distribution_key"))]
const MAPTILER_KEY: &str = dotenvy_macro::dotenv!(
    "DEV_MAPTILER_API_KEY",
    "Missing maptiler API key must be placed in top-level .env file."
);
#[cfg(all(feature = "distribution_key", feature = "ios_config"))]
const MAPTILER_KEY: &str = dotenvy_macro::dotenv!(
    "IOS_MAPTILER_API_KEY",
    "Missing maptiler API key must be placed in top-level .env file."
);
#[cfg(all(feature = "distribution_key", feature = "android_config"))]
const MAPTILER_KEY: &str = dotenvy_macro::dotenv!(
    "ANDROID_MAPTILER_API_KEY",
    "Missing maptiler API key must be placed in top-level .env file."
);

static BLANK_PNG: &[u8] = include_bytes!(env!("BLANK_PNG"));

/// Main map data route. Does the following:
/// - checks if the tile is needed or not (e.g. in an obscured automap region)
/// - checks if the tile is already cached
///     - if the cache is old, attempts to retrieve it, falls back to existing
#[routes]
#[get("/mapdata/{path:.*}")]
#[get("/analyze/mapdata/{path:.*}")]
pub async fn map_data_route(
    path: web::Path<String>,
    req: HttpRequest,
) -> impl Responder {
    if automap_is_on() {
        // if the automap is on and the path matches a tile route, and tile
        // hasn't been visited, prevent the map data from loading
        if let Ok(tilepos) = TileXYZ::try_from(&path) {
            if !tile_has_been_visited(&tilepos) {
                // TODO: figure out how to not get errors for webp (204 doesn't
                // work)
                return no_content_response(&path);
            }
        }
    }

    if let Some(cached) = check_map_cache(&path, &req).await {
        return cached;
    }

    if fetch_is_disabled() {
        return no_content_response(&path);
    }

    let bytes = match fetch_and_cache_from_network(&path).await {
        Some(bytes) => bytes,
        None => return HttpResponse::NotFound().finish(),
    };
    let resp = build_response(&PathBuf::from(path.as_str()), bytes, &req);

    // We fetched new data, so let's check if we should evict old data to keep
    // the cache size within the limit
    tokio::spawn(evict_old_map_data());

    resp
}

pub fn fetch_is_disabled() -> bool {
    AppState::global()
        .persistent
        .lock()
        .unwrap()
        .front
        .as_ref()
        .map(|f| f.map_cache_pref.disable_fetch)
        .unwrap_or(false)
}

/// Response when the requested tile (vector or raster) is outside the explroed
/// region and should be responded with 204 no content or a blank png.
fn no_content_response(path: &str) -> HttpResponse {
    let mut cache_directives = no_caching_directives();
    cache_directives.0.push(CacheDirective::MaxAge(2));
    match PathBuf::from(path).extension().and_then(OsStr::to_str) {
        Some("png") | Some("webp") | Some("jpg") | Some("jpeg") => {
            // browser/maplibre don't like 204 for images, so we return a blank
            // one
            let content_type = "image/png";
            let mut reply = HttpResponse::Ok();
            return reply
                .insert_header((CONTENT_TYPE, content_type))
                .insert_header((ACCESS_CONTROL_ALLOW_ORIGIN, "*"))
                .insert_header(cache_directives)
                .body(BLANK_PNG);
        }
        _ => (),
    };
    let mut reply = HttpResponse::NoContent();
    reply.insert_header((ACCESS_CONTROL_ALLOW_ORIGIN, "*"));
    reply.insert_header(cache_directives);
    reply.finish()
}

const DAY_AS_SECS: u64 = 60 * 60 * 24;
const REFRESH_CACHE_AFTER: u64 = DAY_AS_SECS * 7; // one week

async fn check_map_cache(
    path: &str,
    req: &HttpRequest,
) -> Option<HttpResponse> {
    let fullpath = get_map_cache_dir().join(path);
    // fetch new data if the cached data hasn't been modified in a while
    let metadata = std::fs::metadata(&fullpath).ok()?;
    // SystemTime is not necessarily monotonic, so may error if the file's
    // modified time is in the future, which means it's probably up-to-date
    if let Ok(dur) =
        SystemTime::now().duration_since(metadata.modified().unwrap())
    {
        if dur > std::time::Duration::from_secs(REFRESH_CACHE_AFTER)
            && !fetch_is_disabled()
        {
            debug!("Refreshing cache with new data");
            if let Some(new_bytes) = fetch_and_cache_from_network(path).await {
                return Some(build_response(
                    &PathBuf::from(path),
                    new_bytes,
                    req,
                ));
            }
            // on fetch failure, continue on to retrieve the existing cache item
        }
    }
    std::fs::read(&fullpath).ok().map(|bytes| {
        if bytes.is_empty() {
            // empty file means there was nothing to be found
            HttpResponse::NotFound().finish()
        } else {
            build_response(&fullpath, bytes, req)
        }
    })
}

/// Build the HttpResponse containing map data. Sets the content type based on
/// the file extension, and replaces remote server urls with the backend proxy
/// url based on info in the request.
fn build_response(
    path: &Path,
    mut bytes: Vec<u8>,
    req: &HttpRequest,
) -> HttpResponse {
    substitute_urls(&mut bytes, req, path);
    let content_type = match path.extension().and_then(OsStr::to_str) {
        Some("json") => "application/json",
        Some("pbf") => "application/x-protobuf",
        Some("png") => "image/png",
        Some("jpg") => "image/jpg",
        Some("jpeg") => "image/jpeg",
        Some("webp") => "image/webp",
        Some(unmatched) => unmatched,
        _ => "unknown",
    };
    let mut cache_directives = no_caching_directives();
    cache_directives.0.push(CacheDirective::MaxAge(1000));
    HttpResponse::Ok()
        .insert_header((CONTENT_TYPE, content_type))
        .insert_header((ACCESS_CONTROL_ALLOW_ORIGIN, "*"))
        .insert_header(cache_directives)
        .body(bytes)
}

/// Replace remote server url with backend url in json by mutating the bytes
fn substitute_urls(body: &mut Vec<u8>, req: &HttpRequest, path: &Path) {
    if Some("json") == path.extension().and_then(OsStr::to_str) {
        *body = String::from_utf8(body.clone())
            .unwrap()
            .replace(obfstr!(MAPTILER_URL), &get_backend_url(req))
            .into_bytes();
    }
}

/// Get the url to the map data route with the correct host and scope
fn get_backend_url(req: &HttpRequest) -> String {
    let scope = req
        .uri()
        .path()
        .trim_matches('/')
        .split('/')
        .next()
        .unwrap()
        .to_string();
    let conn_info = req.connection_info();
    let host = conn_info.host();
    format!("http://{host}/{scope}/mapdata")
}

// Shared client between threads so the same TCP connection can be kept alive,
// reducing latency and cost of each request
static CLIENT: OnceCell<reqwest::Client> = OnceCell::const_new();

async fn init_client() -> reqwest::Client {
    obfstr! {
        let user_agent = "WebDriver/A118.35 (iPhone)";
    }
    // We enable the three main compression schemes used. reqwest will auto-
    // decompress the content and strip the related headers, so it's not obvious
    // if compression happens from looking at the response. Set
    // `.connection_verbose(true)` on the ClientBuilder and do RUST_LOG=trace to
    // see relevant output. This was done to confirm that compression was indeed
    // being used to get content over the internet.
    reqwest::ClientBuilder::new()
        .gzip(true)
        .deflate(true)
        .brotli(true)
        .user_agent(user_agent)
        .tcp_keepalive(Some(std::time::Duration::from_secs(60)))
        .build()
        .unwrap()
}

async fn fetch_and_cache_from_network(path: &str) -> Option<Vec<u8>> {
    let now = std::time::Instant::now();
    debug!("retrieving from network: {path}");

    // construct the url to fetch the resource from
    let url = format!(
        "{}/{path}{}{}",
        obfstr!(MAPTILER_URL),
        obfstr!("?key="),
        obfstr!(MAPTILER_KEY)
    );

    let client = CLIENT.get_or_init(init_client).await;

    // response from remote
    // return None if we can't execute the request (e.g. no network)
    let response = client
        .get(&url)
        .header(reqwest::header::ORIGIN, obfstr!("http://127.0.0.1"))
        .send()
        .await
        .ok()?;

    let dest_path = get_map_cache_dir().join(path);
    std::fs::create_dir_all(dest_path.parent().unwrap()).unwrap();

    if !response.status().is_success() {
        info!(
            "Received error status code {:?}, assuming resource doesn't exit",
            response.status()
        );
        // got an error, likely doesn't exist, remember this as an empty file
        std::fs::write(dest_path, b"").unwrap();
        return None;
    }

    let mut body = response.bytes().await.unwrap().to_vec();
    if let Some("json") =
        PathBuf::from(path).extension().and_then(OsStr::to_str)
    {
        // remove instances of the maptiler key before caching
        body = String::from_utf8(body)
            .unwrap()
            .replace(
                &format!("{}{}", obfstr!("?key="), obfstr!(MAPTILER_KEY)),
                "",
            )
            .into_bytes();
    }
    std::fs::write(&dest_path, &body).unwrap(); // cache as-is

    let elapsed_time = now.elapsed();
    debug!("Fetch took {} ms", elapsed_time.as_millis());

    Some(body)
}

// ========== Cache Eviction ========== //

/// Data we need to know about each cached file in order to evict least recently
/// used files and calculate the total disk usage of the cache.
#[derive(Debug)]
struct CachedFile {
    path: PathBuf,     // path to the file
    atime: SystemTime, // last access time for the file
    len: u64,          // length of the file in bytes
}

// excludes multiple evicters from running at the same time
static EVICT_LOCK: Mutex<()> = Mutex::new(());

pub async fn evict_old_map_data() {
    // only enter eviction if there are no other current attempts to do eviction
    let Ok(_evict_guard) = EVICT_LOCK.try_lock() else { return };

    let now = std::time::Instant::now();
    // consider only tiles for deletion, not map styles
    let dir = get_map_cache_dir().join("tiles");
    let cache_pref = AppState::global()
        .persistent
        .lock()
        .unwrap()
        .front
        .as_ref()
        .map(|f| f.map_cache_pref)
        .unwrap_or_default();
    let mut files = vec![];
    for entry in WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
        if let Ok(meta) = entry.metadata() {
            if meta.file_type().is_file()
                && entry.path().extension().and_then(OsStr::to_str)
                    != Some("json")
            {
                // consider for deletion on if it's a file, and it's not json
                // data (since we want to keep the tile spec)
                files.push(CachedFile {
                    path: entry.into_path(),
                    atime: meta.accessed().unwrap(),
                    len: meta.len(),
                });
            }
        }
    }
    // sort files by access time descending (b.partial_cmp(&a))
    files.sort_by(|a, b| {
        b.atime.partial_cmp(&a.atime).unwrap_or(Ordering::Equal)
    });
    let cache_size =
        |files: &Vec<CachedFile>| files.iter().map(|f| f.len).sum::<u64>();
    // debug!("Cache size: {} MB", cache_size(&files) as f64 / 1_000_000.0);
    let initial_size = cache_size(&files);
    while cache_size(&files) > cache_pref.max_size {
        let to_evict = files.pop().unwrap();
        debug!(
            "Evicting file with access time {:?} and path {:?}",
            to_evict.atime, to_evict.path,
        );
        if let Err(e) = std::fs::remove_file(to_evict.path) {
            error!("Failed to evict file: {e}");
        }
    }
    let final_size = cache_size(&files);
    // update the cache size state so it can be displayed in the frontend
    AppState::global()
        .persistent
        .lock()
        .unwrap()
        .back
        .map_cache_size = final_size;
    let elapsed_time = now.elapsed();
    trace!(
        "Map cache size: {} MB -> {} MB. Took {} ms",
        initial_size as f64 / 1_000_000.0,
        final_size as f64 / 1_000_000.0,
        elapsed_time.as_micros() as f64 / 1_000.0
    );
}
