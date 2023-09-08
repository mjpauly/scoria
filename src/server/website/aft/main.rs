use tracing_subscriber::{
    fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter,
};

use website::configuration::get_configuration;
use website::startup::Application;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info")); // default to info level
    let stderr = fmt::Layer::new().with_writer(std::io::stderr).pretty();
    tracing_subscriber::registry()
        .with(env_filter)
        .with(stderr)
        .init(); // also initializes `log` crate compatibility

    let configuration = get_configuration();
    println!("Starting with app config: {:?}", configuration.application);
    let application = Application::build(configuration).await?;
    application.run_until_stopped().await?;
    Ok(())
}
