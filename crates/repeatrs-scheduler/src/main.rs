use repeatrs_scheduler::{ApiResult, Config, initialize_tracing};

#[tokio::main]
async fn main() -> ApiResult<()> {
    Config::init().expect("Failed loading configuration.");

    let _ = initialize_tracing();

    // let app = AppService::build().await?;
    // app.start().await?;

    Ok(())
}
