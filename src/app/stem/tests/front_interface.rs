//! Integration tests for the frontend's interface (user-facing and
//! backend-facing).
//!
//! Make sure to start geckodriver on the command line. It can be installed with
//! cargo install geckodriver

use stem::local::test_setup;

use std::process::Command;

use fantoccini::{Client, ClientBuilder, Locator};
use tokio::time::{sleep, Duration};

#[tokio::test]
async fn ui_interface_tests() -> Result<(), fantoccini::error::CmdError> {
    Command::new("geckodriver")
        .spawn()
        .expect("Couldn't spawn geckodriver");
    sleep(Duration::from_millis(50)).await;
    let c = ClientBuilder::native()
        .connect("http://localhost:4444")
        .await
        .expect("failed to connect to WebDriver");
    // First check that geckodriver works as expected
    // geckodriver_test(&c).await?;

    // Setup testing with the backend
    let server_port = test_setup("end_to_end_works/").await;
    let base_url = format!("http://localhost:{}/123/", server_port);
    c.goto(&base_url).await?;
    // let the webapp load
    sleep(Duration::from_millis(100)).await;

    // UI tests
    simple_navigation(&c, base_url).await?;
    location_config_propagates(&c).await?;
    map_interaction(&c).await?;

    c.close().await
}

#[allow(dead_code)]
async fn geckodriver_test(
    c: &Client,
) -> Result<(), fantoccini::error::CmdError> {
    // first, go to the Wikipedia page for Foobar
    c.goto("https://en.wikipedia.org/wiki/Foobar").await?;
    let url = c.current_url().await?;
    assert_eq!(url.as_ref(), "https://en.wikipedia.org/wiki/Foobar");

    // click "Foo (disambiguation)"
    c.find(Locator::Css(".mw-disambig")).await?.click().await?;

    // click "Foo Lake"
    c.find(Locator::LinkText("Foo Lake")).await?.click().await?;

    let url = c.current_url().await?;
    assert_eq!(url.as_ref(), "https://en.wikipedia.org/wiki/Foo_Lake");

    Ok(())
}

async fn simple_navigation(
    c: &Client,
    base_url: String,
) -> Result<(), fantoccini::error::CmdError> {
    c.find(Locator::Css("#Map")).await?.click().await?;
    assert_url_eq(c, base_url.clone() + "analyze/").await;

    c.find(Locator::Css("#Log")).await?.click().await?;
    assert_url_eq(c, base_url + "sense/").await;

    Ok(())
}

async fn assert_url_eq(c: &Client, desired: String) {
    let url = c.current_url().await.unwrap();
    assert_eq!(url.as_ref(), &desired);
}

/// Check that setting new location config values are readable on Swift-facing
/// interface
async fn location_config_propagates(
    c: &Client,
) -> Result<(), fantoccini::error::CmdError> {
    // first assert the values are set to what we expect, so we know they
    // definitely changed after simulating the ui interaction
    assert!(!stem::get_location_enabled());
    assert!(!stem::get_significant_changes());
    assert!(stem::get_distance_filter() == 5.0);
    assert!(
        stem::get_location_accuracy_mode()
            == stem::common::LocationAccuracyMode::Best
    );

    c.find(Locator::Css("#standard_location"))
        .await?
        .click()
        .await?;

    let dist_filt_elem = c.find(Locator::Css("#distance_filter")).await?;
    dist_filt_elem.send_keys("4").await?;

    c.find(Locator::Css("#accuracy_mode"))
        .await?
        .select_by_index(1)
        .await?;

    assert!(stem::get_location_enabled());
    assert!(stem::get_distance_filter() == 4.0);
    assert!(
        stem::get_location_accuracy_mode()
            == stem::common::LocationAccuracyMode::TenMeters
    );

    // unset standard location service so we can test the significant changes
    c.find(Locator::Css("#standard_location"))
        .await?
        .click()
        .await?;

    c.find(Locator::Css("#significant_changes"))
        .await?
        .click()
        .await?;

    assert!(stem::get_significant_changes());

    Ok(())
}

/// Try updating the map style and time range. Doesn't really assert anything
/// since it's hard to verify the output "looks" correct, but does at least
/// check that we can successfully interact with the page.
async fn map_interaction(
    c: &Client,
) -> Result<(), fantoccini::error::CmdError> {
    sleep(Duration::from_millis(100)).await;
    // Go to the map page
    c.find(Locator::Css("#Map")).await?.click().await?;

    c.find(Locator::Css("#map_style_btn"))
        .await?
        .click()
        .await?;
    c.find(Locator::Css("#opacity")).await?.click().await?;
    c.find(Locator::Css("#marker_size")).await?.click().await?;
    c.find(Locator::Css("#basemap"))
        .await?
        .select_by_index(0)
        .await?;
    c.find(Locator::Css("#marker_color")).await?.click().await?;
    c.find(Locator::Css("#datastream"))
        .await?
        .select_by_index(4)
        .await?;

    c.find(Locator::Css("#time_range_btn"))
        .await?
        .click()
        .await?;
    c.find(Locator::Css("#time_range_week_btn"))
        .await?
        .click()
        .await?;
    c.find(Locator::Css("#time_range_day_btn"))
        .await?
        .click()
        .await?;
    c.find(Locator::Css("#time_range_today_btn"))
        .await?
        .click()
        .await?;
    c.find(Locator::Css("#start")).await?.click().await?;
    c.find(Locator::Css("#end")).await?.click().await?;

    Ok(())
}
