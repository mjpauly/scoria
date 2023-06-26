use website::configuration::get_configuration;
use website::startup::Application;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let configuration = get_configuration();
    println!("Starting with app config: {:?}", configuration.application);
    let application = Application::build(configuration).await?;
    application.run_until_stopped().await?;
    Ok(())
}
