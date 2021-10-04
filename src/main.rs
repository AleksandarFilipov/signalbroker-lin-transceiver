use std::{error::Error, sync::Arc};

use config::Config;
use lin_udp_client::LinUdpClient;
use record::Record;
use tokio::sync::Mutex;
use tokio::time::sleep;
use std::time::Duration;
use records::Records;

mod config;
mod lin_udp_client;
mod record;
mod records;

const RIB_ID: u8 = 0x01;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let records: Arc<Mutex<Records>> = Arc::new(Mutex::new(Records::new()));

    let config = Arc::new(Config::new(RIB_ID, Arc::clone(&records)));
    config.init().await?;

    let lin_udp_client = LinUdpClient::new(Arc::clone(&config), Arc::clone(&records));

    tokio::spawn({
        let config = Arc::clone(&config);
        async move {
            config.run().await;
        }
    });

    // sleep(Duration::from_secs(10)).await;

    lin_udp_client.run().await;
    Ok(())
}
