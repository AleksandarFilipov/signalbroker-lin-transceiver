use clap::{App, Arg};
use std::{collections::HashMap, error::Error, net::UdpSocket, sync::Arc};

use config::Config;
use lin_udp_client::LinUdpClient;
use records::Records;
use tokio::sync::Mutex;

mod config;
mod lin_udp_client;
mod record;
mod records;

const RIB_ID: u8 = 0x04;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let _matches = App::new("My Super Program")
        .version("1.0")
        .author("Niclas Lind. <niclas.lind@volvocars.com>")
        .about("Does awesome things")
        .arg(
            Arg::with_name("device-port")
                .short("d")
                .long("device")
                .help("Uart Port")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("ID")
                .short("id")
                .long("device_id")
                .help("RIB ID")
                .takes_value(true),
        )
        .get_matches();

    let records: Arc<Mutex<Records>> = Arc::new(Mutex::new(Records::new()));
    let config = Arc::new(Config::new(RIB_ID, Arc::clone(&records)));
    config.init().await?;

    let signal_broker_udp_listener = UdpSocket::bind("0.0.0.0:4000")?;

    tokio::spawn({
        let config = config.clone();

        let mut configs = HashMap::new();
        configs.insert(RIB_ID, config);
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

    let lin_udp_client =
        LinUdpClient::new(Arc::clone(&config), Arc::clone(&records), "/dev/serial0");

    tokio::spawn({
        let config = Arc::clone(&config);
        async move {
            config.run().await;
        }
    });

    lin_udp_client.run().await;
    Ok(())
}
