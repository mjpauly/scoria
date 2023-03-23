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
    let base_url = format!("http://localhost:{}", server_port);
    c.goto(&base_url).await?;
    // let the webapp load
    sleep(Duration::from_millis(100)).await;

    // UI tests
    simple_navigation(&c, base_url).await?;

    c.close().await
}

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
    c.find(Locator::Css("#Analyze")).await?.click().await?;
    assert_url_eq(c, base_url.clone() + "/analyze").await;

    c.find(Locator::Css("#Test")).await?.click().await?;
    assert_url_eq(c, base_url.clone() + "/test_page").await;

    c.find(Locator::Css("#Sense")).await?.click().await?;
    assert_url_eq(c, base_url + "/").await;

    Ok(())
}

async fn assert_url_eq(c: &Client, desired: String) {
    let url = c.current_url().await.unwrap();
    assert_eq!(url.as_ref(), &desired);
}
