extern crate stem;

use stem::common::{Location, ToBack, ToFront};
use stem::local::test_setup;

use futures_util::{stream::TryStreamExt, StreamExt};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

/// Runner for all the tests. Because it's a multithreaded application with
/// state, we run these tests sequentially.
#[tokio::test]
async fn integration_tests() {
    let url = setup("integration_tests/").await;
    log_location_sends_data_to_ui(&url).await;
    backend_sends_state_when_requested(&url).await;
}

async fn setup(dir: &str) -> String {
    let port = test_setup(dir).await;
    format!("ws://127.0.0.1:{}/ws", port)
}

async fn log_location_sends_data_to_ui(url: &str) {
    let (ws_stream, _) = connect_async(url).await.expect("Failed to connect");
    let (_write, mut read) = ws_stream.split();

    stem::core::log_location(0., 1., 2., 3., 4., 100).await;

    let msg = read.try_next().await.unwrap().unwrap();
    assert!(msg.is_binary());

    let decoded = bincode::deserialize::<ToFront>(&msg.into_data()).unwrap();

    let expected = Location {
        lat: 0.,
        lon: 1.,
        accuracy: 2.,
        speed: 3.,
        course: 4.,
        datetime: time::OffsetDateTime::from_unix_timestamp(100).unwrap(),
    };
    assert_eq!(decoded, ToFront::LastLocation(expected));
}

// TODO: finish
async fn backend_sends_state_when_requested(url: &str) {
    // let url = setup("backend_sends_state_when_requested/").await;
    let (ws_stream, _) = connect_async(url).await.expect("Failed to connect");
    let (mut write, mut read) = ws_stream.split();
    assert_eq!(1, 1);
}
