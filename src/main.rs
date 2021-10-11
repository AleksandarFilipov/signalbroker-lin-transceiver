use std::{collections::HashMap, error::Error, net::UdpSocket, sync::Arc};

use config::Config;
use lin_udp_client::LinUdpClient;
use records::Records;
use serde_derive::Deserialize;
use std::fs::File;
use std::io::prelude::*;
use tokio::sync::Mutex;

mod config;
mod lin_udp_client;
mod record;
mod records;

#[derive(Debug, Deserialize)]
struct UartConfig {
    global_string: Option<String>,
    num_ports: Option<u64>,
    lin_ports: Vec<LinConfig>,
}

#[derive(Debug, Deserialize)]
struct LinConfig {
    uart: String,
    rib_id: u8,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut file = File::open("lin_config.toml").expect("Unable to open the file");
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .expect("Unable to read the file");

    let decoded: UartConfig = toml::from_str(&contents).unwrap();

    let mut configs = HashMap::new();

    for port in decoded.lin_ports {
        let records: Arc<Mutex<Records>> = Arc::new(Mutex::new(Records::new()));
        let config = Arc::new(Config::new(port.rib_id, Arc::clone(&records)));
        config.init().await?;

        configs.insert(port.rib_id, Arc::clone(&config));

        tokio::spawn({
            let config = Arc::clone(&config);

            async move {
                config.run().await;
            }
        });

        tokio::spawn({
            let config = Arc::clone(&config);
            let lin_udp_client = LinUdpClient::new(config, records, &port.uart);
            async move {
                lin_udp_client.run().await;
            }
        });
    }

    let signal_broker_udp_listener = UdpSocket::bind("0.0.0.0:4000")?;
    tokio::spawn({
        async move {
            loop {
                let mut signal_broker_data = vec![0; 128];
                let (len, ip_server) = signal_broker_udp_listener
                    .recv_from(&mut signal_broker_data)
                    .unwrap();

                let id = signal_broker_data[1];
                let config = configs.get(&id).unwrap();
                config.set_server_data(&signal_broker_data).await;
                config.set_server_ip_address(ip_server.ip()).await;

                println!(
                    "Recv {} bytes from {} with data {:#?}",
                    len,
                    ip_server,
                    &signal_broker_data[..=len]
                );
            }
        }
    });

    loop {}

    Ok(())
}
