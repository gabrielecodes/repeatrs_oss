use repeatrs_scheduler::{Config, ServiceResult, initialize_tracing};

#[tokio::main]
async fn main() -> ServiceResult<()> {
    Config::init().expect("Failed loading configuration.");

    let _ = initialize_tracing();

    // let app = AppService::build().await?;
    // app.start().await?;

    Ok(())
}
