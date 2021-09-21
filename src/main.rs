use std::error::Error;

use config::Config;

mod config;

const RIB_ID: u8 = 0x01;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let config = Config::new(RIB_ID).await;
    config.run().await;

    Ok(())
}
