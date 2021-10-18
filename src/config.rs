use std::{
    net::{IpAddr, Ipv4Addr},
    sync::Arc,
    time::Duration,
};

use byteorder::{BigEndian, ByteOrder};
use signalbroker_lin_transceiver_rp::{
    ConfigHash, Counter, HeartBeatModes, NodeMode, ServerMessageOffsets, UdpPort, CLIENT_PORT,
    HEADER, HEART_BEAT, HOST_PORT, MESSAGE_SIZES, NAD, NODE_MODE,
};
use tokio::{net::UdpSocket, sync::Mutex, time::sleep};
use tracing::debug;

use crate::record::Record;
use crate::records::Records;
use std::error::Error;
use std::ops::Sub;
use std::time::SystemTime;

pub struct Config {
    rib_id: u8,
    udp_ports: Arc<Mutex<UdpPort>>,
    udp_server_send_client: Arc<Mutex<Option<UdpSocket>>>,
    heart_beat_period: Duration,
    ip_address_server: Arc<Mutex<IpAddr>>,
    hashes: ConfigHash,
    server_data: Arc<Mutex<Vec<u8>>>,
    new_data: Arc<Mutex<bool>>,
    records: Arc<Mutex<Records>>,
    counters: Arc<Mutex<Counter>>,
    node_mode: Arc<Mutex<NodeMode>>,
    nad: Arc<Mutex<u8>>,
    received_ip: Arc<Mutex<bool>>,
    latest_updated_time: Arc<Mutex<SystemTime>>,
}

impl Config {
    pub fn new(rib_id: u8, records: Arc<Mutex<Records>>) -> Self {
        Self {
            rib_id,
            udp_ports: Arc::new(Mutex::new(UdpPort::new())),
            udp_server_send_client: Arc::new(Mutex::new(None)),
            heart_beat_period: Duration::from_millis(2000),
            ip_address_server: Arc::new(Mutex::new(IpAddr::V4(Ipv4Addr::new(255, 255, 255, 255)))),
            hashes: ConfigHash::new(),
            server_data: Arc::new(Mutex::new(vec![0u8; 128])),
            new_data: Arc::new(Mutex::new(false)),
            records,
            counters: Arc::new(Mutex::new(Counter::new())),
            node_mode: Arc::new(Mutex::new(NodeMode::Undefined)),
            nad: Arc::new(Mutex::new(0)),
            received_ip: Arc::new(Mutex::new(false)),
            latest_updated_time: Arc::new(Mutex::new(
                SystemTime::now().sub(Duration::from_secs(5)),
            )),
        }
    }

    pub async fn init(&self) -> Result<(), Box<dyn Error>> {
        *self.udp_server_send_client.lock().await = Some(UdpSocket::bind("0.0.0.0:0").await?);

        let udp_server_send_client_port = self.udp_ports.lock().await.udp_server_config_port;
        let udp_server_send_client_address = *self.ip_address_server.lock().await;
        let address = format!(
            "{}:{}",
            udp_server_send_client_address, udp_server_send_client_port
        );

        // Add permission to broadcast to entire network and connect
        if let Some(udp_server_send) = &*self.udp_server_send_client.lock().await {
            udp_server_send.set_broadcast(true)?;
            udp_server_send.connect(&address).await?;
        }

        println!("Init completed");
        Ok(())
    }

    pub async fn increment_un_synched_packages(&self) {
        self.counters.lock().await.unsynched_packages += 1;
    }

    pub async fn set_synch_count(&self, val: u16) {
        self.counters.lock().await.sync_count = val;
    }

    pub async fn increment_synched_packages(&self) {
        self.counters.lock().await.synched_packages += 1;
    }

    pub async fn increment_rx_over_lin(&self) {
        self.counters.lock().await.rx_over_lin += 1;
    }

    pub async fn increment_rx_over_udp(&self) {
        self.counters.lock().await.rx_over_udp += 1;
    }

    pub async fn increment_tx_over_udp(&self) {
        self.counters.lock().await.tx_over_udp += 1;
    }

    pub async fn increment_tx_over_lin(&self) {
        self.counters.lock().await.tx_over_lin += 1;
    }

    pub async fn host_ip(&self) -> IpAddr {
        *self.ip_address_server.lock().await
    }

    pub async fn host_port(&self) -> u16 {
        self.udp_ports.lock().await.udp_lin_host_port
    }

    pub async fn client_port(&self) -> u16 {
        self.udp_ports.lock().await.udp_lin_client_port
    }

    pub async fn received_ip(&self) -> bool {
        *self.received_ip.clone().lock().await
    }

    pub async fn node_mode(&self) -> NodeMode {
        self.node_mode.lock().await.clone()
    }

    pub async fn set_server_data(&self, data: &[u8]) {
        self.server_data.lock().await.clone_from_slice(data);
        *self.new_data.lock().await = true;
    }

    pub async fn set_server_ip_address(&self, address: IpAddr) {
        *self.ip_address_server.lock().await = address;
        *self.received_ip.lock().await = true;
    }

    pub async fn run(&self) -> Result<(), Box<dyn Error>> {
        self.send_heartbeat().await?;
        self.parse_server_message().await;
        self.verify_config().await?;
        Ok(())
    }

    async fn send_heartbeat(&self) -> Result<(), Box<dyn Error>> {
        let time = std::time::SystemTime::now();
        let mut latest_time = self.latest_updated_time.lock().await;
        if time.duration_since(*latest_time)? > self.heart_beat_period {
            *latest_time = time;

            let device_hash = self.hashes.device_hash.lock().await.to_be_bytes();
            let mut counters = self.counters.lock().await;
            let rx_over_lin = counters.rx_over_lin.to_be_bytes();
            let tx_over_lin = counters.tx_over_lin.to_be_bytes();
            let rx_over_udp = counters.rx_over_udp.to_be_bytes();
            let tx_over_udp = counters.tx_over_udp.to_be_bytes();
            let sync_count = counters.sync_count.to_be_bytes();
            let unsynched_packages = counters.unsynched_packages.to_be_bytes();
            let synched_packages = counters.synched_packages.to_be_bytes();

            let mut data = Vec::with_capacity(30);
            data.push(HEADER); // HEADER
            data.push(self.rib_id); // RIB_ID
            data.push(device_hash[0]); // LAST_HASH_LOW
            data.push(device_hash[1]); // LAST_HASH_HIGH
            data.push(HEART_BEAT); // HEARTBEAT

            // Payload length
            data.push(0x00);
            data.push(0x15);

            // RxOverLin
            data.push(HeartBeatModes::RxLin.into());
            data.push(rx_over_lin[0]);
            data.push(rx_over_lin[1]);

            // TxOverLin
            data.push(HeartBeatModes::TxLin.into());
            data.push(tx_over_lin[0]);
            data.push(tx_over_lin[1]);

            // RxOverUDP
            data.push(HeartBeatModes::RxUdp.into());
            data.push(rx_over_udp[0]);
            data.push(rx_over_udp[1]);

            // TxOverUDP
            data.push(HeartBeatModes::TxUdp.into());
            data.push(tx_over_udp[0]);
            data.push(tx_over_udp[1]);

            // SyncCount
            data.push(HeartBeatModes::SyncCount.into());
            data.push(sync_count[0]);
            data.push(sync_count[1]);

            // UnSynchedPackages
            data.push(HeartBeatModes::UnSynchedPackages.into());
            data.push(unsynched_packages[0]);
            data.push(unsynched_packages[1]);

            // SynchedPackages
            data.push(HeartBeatModes::SynchedPackages.into());
            data.push(unsynched_packages[0]);
            data.push(synched_packages[1]);

            if let Some(udp_sender) = &*self.udp_server_send_client.lock().await {
                let len = udp_sender.send(&data).await?;
                debug!("Wrote {} bytes as heartbeat", len);
            }

            counters.clear_counters();
        }
        Ok(())
    }

    async fn verify_config(&self) -> Result<(), Box<dyn Error>> {
        let mut good_config = false;

        while !good_config {
            let device_hash;
            let client_port_hash: u16;
            let host_port_hash: u16;
            let node_mode_hash: u16;
            let message_sizes_hash: u16;
            let nad_hash: u16;

            // prevent deadlocks by only held the lock in this scope.
            {
                device_hash = *self.hashes.device_hash.lock().await;
                client_port_hash = *self.hashes.client_port_hash.lock().await;
                host_port_hash = *self.hashes.host_port_hash.lock().await;
                node_mode_hash = *self.hashes.node_mode_hash.lock().await;
                message_sizes_hash = *self.hashes.message_sizes_hash.lock().await;
                nad_hash = *self.hashes.nad_hash.lock().await;
            }

            good_config = true;

            if host_port_hash != device_hash {
                good_config = false;
                self.request_config_item(HOST_PORT).await?;
            } else if client_port_hash != device_hash {
                good_config = false;
                self.request_config_item(CLIENT_PORT).await?;
            } else if node_mode_hash != device_hash {
                good_config = false;
                self.request_config_item(NODE_MODE).await?;
            } else if message_sizes_hash != device_hash {
                good_config = false;
                self.request_config_item(MESSAGE_SIZES).await?;
            } else if nad_hash != device_hash {
                good_config = false;
                self.request_config_item(NAD).await?;
            }

            if !good_config {
                sleep(Duration::from_millis(50)).await;
                self.parse_server_message().await;
            }
        }
        Ok(())
    }

    async fn request_config_item(&self, item: u8) -> Result<(), Box<dyn Error>> {
        let mut data = Vec::with_capacity(10);
        let device_hash = self.hashes.device_hash.lock().await.to_be_bytes();

        data.push(HEADER);
        data.push(self.rib_id);
        data.push(device_hash[0]); // LAST_HASH_LOW
        data.push(device_hash[1]); // LAST_HASH_HIGH
        data.push(item);
        data.push(0x00);
        data.push(0x00);

        println!("Request config {}", item);

        self.udp_server_send_client
            .lock()
            .await
            .as_ref()
            .unwrap()
            .send(&data)
            .await
            .unwrap();
    }

    async fn parse_server_message(&self) {
        let device_hash;
        {
            device_hash = *self.hashes.device_hash.lock().await;
        }

        let data = self.server_data.lock().await;
        let mut new_data = self.new_data.lock().await;

        if !(*new_data) {
            return;
        }

        *new_data = false;

        // if the first bytes isn't the HEADER identifier or if the message isn't intended for this ID
        // if HEADER != data[ServerMessageOffsets::Header as usize]
        //     || self.rib_id != data[ServerMessageOffsets::RibId as usize]
        // {
        //     println!("Didn't match expected");
        // }

        *self.hashes.device_hash.lock().await = BigEndian::read_u16(
            &data[(ServerMessageOffsets::HashHigh as usize)
                ..=(ServerMessageOffsets::HashLow as usize)],
        );

        if 0 == data[ServerMessageOffsets::Identifier as usize] {
            return;
        }

        let message_size = BigEndian::read_u16(
            &data[(ServerMessageOffsets::PayloadSizeHigh as usize)
                ..=(ServerMessageOffsets::PayloadSizeLow as usize)],
        );

        match data[ServerMessageOffsets::Identifier as usize] {
            HOST_PORT => {
                println!("Received hostport config");
                if message_size != 2 {
                    return;
                }
                let host_port = BigEndian::read_u16(
                    &data[(ServerMessageOffsets::PayloadStart as usize)
                        ..=(ServerMessageOffsets::PayloadStart as usize + 1)],
                );
                self.udp_ports.lock().await.udp_lin_host_port = host_port;
                *self.hashes.host_port_hash.lock().await = device_hash;
            }
            CLIENT_PORT => {
                println!("Received clientport config");
                if message_size != 2 {
                    return;
                }
                let client_port = BigEndian::read_u16(
                    &data[(ServerMessageOffsets::PayloadStart as usize)
                        ..=(ServerMessageOffsets::PayloadStart as usize + 1)],
                );
                self.udp_ports.lock().await.udp_lin_client_port = client_port;
                *self.hashes.client_port_hash.lock().await = device_hash;
            }
            MESSAGE_SIZES => {
                println!("Received message_sizes config");

                let count = ServerMessageOffsets::PayloadStart as usize;
                let record_size: usize = 3;
                let record_entries = message_size as usize / record_size;
                println!("Record entries {}", record_entries);

                let records = (count..count + (record_entries * record_size))
                    .step_by(record_size)
                    .map(|v| {
                        let id = data[v];
                        let size = data[v + 1];
                        let master = data[v + 2];
                        Record::new(id, size, master)
                    })
                    .collect::<Vec<Record>>();
                println!("Records: {:#?}", records);

                self.records.lock().await.set_list(records);
                *self.hashes.message_sizes_hash.lock().await = device_hash;
            }
            NODE_MODE => {
                println!("Received node_mode config");
                if message_size != 1 {
                    return;
                }
                let node_mode = data[(ServerMessageOffsets::PayloadStart) as usize];
                *self.node_mode.lock().await = node_mode.into();
                *self.hashes.node_mode_hash.lock().await = device_hash;
            }
            NAD => {
                println!("Received nad config");
                if message_size != 1 {
                    return;
                }
                let nad = data[(ServerMessageOffsets::PayloadStart) as usize];
                println!("NAD: {}", nad);
                *self.nad.lock().await = nad;
                *self.hashes.nad_hash.lock().await = device_hash;
            }
            _ => {}
        }
    }
}
