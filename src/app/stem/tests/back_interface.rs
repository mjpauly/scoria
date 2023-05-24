//! Integration tests for the backend's interface (Swift-facing and frontend-
//! facing).

use common::state::MapState;
use common::{
    FrontState, Location, LocationAccuracyMode, StandardLocationConfig,
    TimeRange, ToBack, ToFront, UserConfig,
};

use crate::setup;

use futures_util::{stream::TryStreamExt, SinkExt, StreamExt};
use rusty_fork::rusty_fork_test;
use tokio::runtime::Runtime;
use tokio::time::{sleep, Duration};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

/// Creates a new tokio runtime and blocks on the future provided.
fn run_test<F: std::future::Future>(fut: F) -> F::Output {
    Runtime::new().unwrap().block_on(fut)
}

// Fork a new process for each test since the backend state would otherwise be
// shared.
// To do later: turn this into a proc_macro_attribute
rusty_fork_test! {
    #[test]
    fn log_location_sends_data_to_ui() {
        run_test(log_location_sends_data_to_ui_impl());
    }

    #[test]
    fn set_location_config_changes_backend_state() {
        run_test(set_location_config_changes_backend_state_impl());
    }

    #[test]
    fn backend_sends_state_when_requested() {
        run_test(backend_sends_state_when_requested_impl());
    }

    #[test]
    fn backend_sends_location_time_range_when_requested() {
        run_test(backend_sends_location_time_range_when_requested_impl());
    }

    #[test]
    fn backend_server_websocket_inaccessible_after_app_background() {
        run_test(
            backend_server_websocket_inaccessible_after_app_background_impl()
        );
    }
}

async fn log_location_sends_data_to_ui_impl() {
    let url = setup("log_location_sends_data_to_ui/").await;
    let (ws_stream, _) = connect_async(url).await.expect("Failed to connect");
    let (_write, mut read) = ws_stream.split();

    let timestamp = 100;
    stem::core::log_location(0., 1., 2., 3., 4., timestamp).await;

    let msg = read.try_next().await.unwrap().unwrap();
    assert!(msg.is_binary());

    let decoded = bincode::deserialize::<ToFront>(&msg.into_data()).unwrap();

    let expected = Location {
        lat: 0.,
        lon: 1.,
        accuracy: 2.,
        speed: 3.,
        course: 4.,
        datetime: time::OffsetDateTime::from_unix_timestamp(timestamp).unwrap(),
    };
    assert_eq!(decoded, ToFront::LastLocation(expected));
}

async fn set_location_config_changes_backend_state_impl() {
    let url = setup("set_location_config_changes_backend_state/").await;
    let (ws_stream, _) = connect_async(url).await.expect("Failed to connect");
    let (mut write, _read) = ws_stream.split();

    // let new_config = UserConfig {
    let front_state = FrontState {
        location_config: UserConfig {
            standard_config: StandardLocationConfig {
                distance_filter: 4.0,
                accuracy_mode: LocationAccuracyMode::Best,
            },
            ..Default::default()
        },
        route: Default::default(),
        use_epsln_tile_server: Default::default(),
        map: MapState {
            time_range: TimeRange {
                start: time::OffsetDateTime::now_utc(),
                end: time::OffsetDateTime::now_utc(),
            },
            style: Default::default(),
            filters: vec![],
            view_pos: Default::default(),
        },
    };
    let msg = ToBack::SetFrontState(front_state.clone());
    let encoded = bincode::serialize(&msg).unwrap();
    write.send(Message::binary(encoded)).await.unwrap();

    // Wait for the message to propagate
    sleep(Duration::from_millis(50)).await;

    let persisted = stem::app_state::AppState::global()
        .persistent
        .lock()
        .unwrap()
        .front
        .clone();
    assert_eq!(front_state, persisted.unwrap());
}

async fn backend_sends_state_when_requested_impl() {
    let url = setup("backend_sends_state_when_requested/").await;
    let (ws_stream, _) = connect_async(url).await.expect("Failed to connect");
    let (mut write, mut read) = ws_stream.split();

    // first let's store a location
    stem::core::log_location(0., 1., 2., 3., 4., 100).await;

    for msg in [ToBack::GetFrontState, ToBack::GetBackState] {
        let encoded = bincode::serialize(&msg).unwrap();
        write.send(Message::binary(encoded)).await.unwrap();
    }

    let mut messages = Vec::new();
    for _ in 0..3 {
        let msg = read.try_next().await.unwrap().unwrap();
        let decoded =
            bincode::deserialize::<ToFront>(&msg.into_data()).unwrap();
        messages.push(decoded);
    }

    // We find the message within the vector since the order is not guaranteed.
    messages
        .iter()
        .position(|x| matches!(*x, ToFront::FrontState(_)))
        .expect("Did not receive FrontState");
    messages
        .iter()
        .position(|x| matches!(*x, ToFront::BackState(_)))
        .expect("Did not receive BackState");
    messages
        .iter()
        .position(|x| matches!(*x, ToFront::LastLocation(_)))
        .expect("Did not receive LastLocation state");
}

async fn backend_sends_location_time_range_when_requested_impl() {
    let url = setup("backend_sends_location_time_range_when_requested/").await;
    let (ws_stream, _) = connect_async(url).await.expect("Failed to connect");
    let (mut write, mut read) = ws_stream.split();

    // day time range
    let now = time::OffsetDateTime::now_local().unwrap();
    let start = now.replace_time(time::Time::MIDNIGHT);
    let end = now.replace_time(time::Time::from_hms(23, 59, 59).unwrap());
    let time_range = TimeRange { start, end };

    let msg = ToBack::GetLocationTimeRange(time_range.clone());
    let encoded = bincode::serialize(&msg).unwrap();
    write.send(Message::binary(encoded)).await.unwrap();

    let msg = read.try_next().await.unwrap().unwrap();
    let decoded = bincode::deserialize::<ToFront>(&msg.into_data()).unwrap();

    if let ToFront::LocationTimeRange(tr, _) = decoded {
        assert_eq!(tr, time_range);
    } else {
        panic!("Didn't receive LocationTimeRange from backend.");
    }
}

/// Check that the backend server websocket is inaccessible after the app goes
/// into the background and the server is shutdown.
async fn backend_server_websocket_inaccessible_after_app_background_impl() {
    let url =
        setup("backend_server_websocket_inaccessible_after_app_background/")
            .await;

    // Shutdown the server as would happen when the app goes to background
    stem::server::shutdown().await;

    let result = connect_async(url).await;
    assert!(result.is_err());
}
