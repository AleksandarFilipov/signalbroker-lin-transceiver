use std::{
    net::{IpAddr, Ipv4Addr},
    sync::Arc,
    time::Duration,
};

use byteorder::{BigEndian, ByteOrder};
use tokio::{net::UdpSocket, sync::Mutex, time::sleep};

pub const HEADER: u8 = 0x04;
pub const HOST_PORT: u8 = 0x01;
pub const CLIENT_PORT: u8 = 0x02;
pub const MESSAGE_SIZES: u8 = 0x04;
pub const NODE_MODE: u8 = 0x08;
pub const HEART_BEAT: u8 = 0x10;
pub const NAD: u8 = 0x20;
pub const LOGGER: u8 = 0x60;

#[repr(u8)]
pub enum ServerMessageOffsets {
    Header,
    RibId,
    HashHigh,
    HashLow,
    Identifier,
    PayloadSizeHigh,
    PayloadSizeLow,
    PayloadStart,
}

#[repr(u8)]
pub enum HeartBeatModes {
    TxLin,
    RxLin,
    TxUdp,
    RxUdp,
    SyncCount,
    UnSynchedPackages,
    SynchedPackages,
}

impl From<HeartBeatModes> for u8 {
    fn from(val: HeartBeatModes) -> Self {
        match val {
            HeartBeatModes::TxLin => 0x1,
            HeartBeatModes::RxLin => 0x2,
            HeartBeatModes::TxUdp => 0x3,
            HeartBeatModes::RxUdp => 0x4,
            HeartBeatModes::SyncCount => 0x5,
            HeartBeatModes::UnSynchedPackages => 0x6,
            HeartBeatModes::SynchedPackages => 0x7,
        }
    }
}

impl From<ServerMessageOffsets> for u8 {
    fn from(val: ServerMessageOffsets) -> Self {
        match val {
            ServerMessageOffsets::Header => 0x0,
            ServerMessageOffsets::RibId => 0x1,
            ServerMessageOffsets::HashHigh => 0x2,
            ServerMessageOffsets::HashLow => 0x3,
            ServerMessageOffsets::Identifier => 0x4,
            ServerMessageOffsets::PayloadSizeHigh => 0x5,
            ServerMessageOffsets::PayloadSizeLow => 0x6,
            ServerMessageOffsets::PayloadStart => 0x7,
        }
    }
}

struct ConfigHash {
    device_hash: Arc<Mutex<u16>>,
    client_port_hash: Arc<Mutex<u16>>,
    host_port_hash: Arc<Mutex<u16>>,
    node_mode_hash: Arc<Mutex<u16>>,
    message_sizes_hash: Arc<Mutex<u16>>,
    nad_hash: Arc<Mutex<u16>>,
}

impl ConfigHash {
    fn new() -> Self {
        let defualt_hash = 0xFFFF_u16;

        Self {
            device_hash: Arc::new(Mutex::new(defualt_hash)),
            client_port_hash: Arc::new(Mutex::new(defualt_hash)),
            host_port_hash: Arc::new(Mutex::new(defualt_hash)),
            node_mode_hash: Arc::new(Mutex::new(defualt_hash)),
            message_sizes_hash: Arc::new(Mutex::new(defualt_hash)),
            nad_hash: Arc::new(Mutex::new(defualt_hash)),
        }
    }
}

struct UdpPort {
    udp_server_config_port: Arc<Mutex<u16>>,
    udp_target_config_port: Arc<Mutex<u16>>,
    udp_lin_host_port: Arc<Mutex<u16>>,
    udp_lin_client_port: Arc<Mutex<u16>>,
}

impl UdpPort {
    fn new() -> Self {
        Self {
            udp_server_config_port: Arc::new(Mutex::new(4001)),
            udp_target_config_port: Arc::new(Mutex::new(4000)),
            udp_lin_client_port: Arc::new(Mutex::new(0)),
            udp_lin_host_port: Arc::new(Mutex::new(0)),
        }
    }
}

pub struct Config {
    rib_id: u8,
    udp_ports: UdpPort,
    udp_server_listen_client: Arc<Mutex<UdpSocket>>,
    udp_server_send_client: Arc<Mutex<UdpSocket>>,
    heart_beat_period: u64,
    ip_address_server: IpAddr,
    hashes: ConfigHash,
    server_data: Arc<Mutex<Vec<u8>>>,
    new_data: Arc<Mutex<bool>>,
}

impl Config {
    pub async fn new(rib_id: u8) -> Self {
        Self {
            rib_id,
            udp_ports: UdpPort::new(),
            udp_server_listen_client: Arc::new(Mutex::new(
                UdpSocket::bind("[::]:4000").await.unwrap(),
            )),
            udp_server_send_client: Arc::new(Mutex::new(UdpSocket::bind("[::]:0").await.unwrap())),
            heart_beat_period: 2500,
            ip_address_server: IpAddr::V4(Ipv4Addr::new(255, 255, 255, 255)),
            hashes: ConfigHash::new(),
            server_data: Arc::new(Mutex::new(vec![0u8; 128])),
            new_data: Arc::new(Mutex::new(false)),
        }
    }

    pub async fn run(&self) {
        self.udp_server_send_client
            .lock()
            .await
            .connect("localhost:4001")
            .await
            .unwrap();

        tokio::spawn({
            let client = self.udp_server_listen_client.clone();
            let server_data = self.server_data.clone();
            let new_data = self.new_data.clone();
            async move {
                loop {
                    let mut data = vec![0; 128];
                    let len = client.lock().await.recv(&mut data).await.unwrap();
                    *server_data.lock().await = data.clone();
                    *new_data.lock().await = true;
                    println!("Received {} bytes with data {:#?}", len, &data[..len]);
                }
            }
        });

        loop {
            let mut device_hash = self.hashes.device_hash.lock().await;

            self.send_heartbeat(*device_hash).await;
            self.parse_server_message(&mut device_hash).await;
            self.verify_config(&mut device_hash).await;
        }
    }

    async fn send_heartbeat(&self, device_hash: u16) {
        sleep(Duration::from_millis(self.heart_beat_period)).await;

        let mut data = Vec::default();
        data.push(HEADER); // HEADER
        data.push(self.rib_id); // RIB_ID
        data.push(((device_hash >> 8) & 0xFF) as u8); // LAST_HASH_LOW
        data.push((device_hash & 0xFF) as u8); // LAST_HASH_HIGH
        data.push(HEART_BEAT); // HEARTBEAT

        // Payload length
        data.push(0x00);
        data.push(0x15);

        // RxOverLin
        data.push(HeartBeatModes::RxLin.into());
        data.push(0x00);
        data.push(0x00);

        // TxOverLin
        data.push(HeartBeatModes::TxLin.into());
        data.push(0x00);
        data.push(0x00);

        // RxOverUDP
        data.push(HeartBeatModes::RxUdp.into());
        data.push(0x00);
        data.push(0x00);

        // TxOverUDP
        data.push(HeartBeatModes::TxUdp.into());
        data.push(0x00);
        data.push(0x00);

        // SyncCount
        data.push(HeartBeatModes::SyncCount.into());
        data.push(0x00);
        data.push(0x00);

        // UnSynchedPackages
        data.push(HeartBeatModes::UnSynchedPackages.into());
        data.push(0x00);
        data.push(0x00);

        // SynchedPackages
        data.push(HeartBeatModes::SynchedPackages.into());
        data.push(0x00);
        data.push(0x00);

        self.udp_server_send_client
            .lock()
            .await
            .send(&data)
            .await
            .unwrap();
    }

    async fn verify_config(&self, device_hash: &mut u16) {
        let mut good_config = false;

        while !good_config {
            let client_port_hash: u16;
            let host_port_hash: u16;
            let node_mode_hash: u16;
            let message_sizes_hash: u16;
            let nad_hash: u16;

            // prevent deadlocks by only held the lock in this scope.
            {
                client_port_hash = *self.hashes.client_port_hash.lock().await;
                host_port_hash = *self.hashes.host_port_hash.lock().await;
                node_mode_hash = *self.hashes.node_mode_hash.lock().await;
                message_sizes_hash = *self.hashes.message_sizes_hash.lock().await;
                nad_hash = *self.hashes.nad_hash.lock().await;
            }

            good_config = true;

            if host_port_hash != *device_hash {
                good_config = false;
                self.request_config_item(HOST_PORT, *device_hash).await;
            } else if client_port_hash != *device_hash {
                good_config = false;
                self.request_config_item(CLIENT_PORT, *device_hash).await;
            } else if node_mode_hash != *device_hash {
                good_config = false;
                self.request_config_item(NODE_MODE, *device_hash).await;
            } else if message_sizes_hash != *device_hash {
                good_config = false;
                self.request_config_item(MESSAGE_SIZES, *device_hash).await;
            } else if nad_hash != *device_hash {
                good_config = false;
                self.request_config_item(NAD, *device_hash).await;
            }

            if !good_config {
                sleep(Duration::from_millis(200)).await;
                self.parse_server_message(device_hash).await;
            }
        }
    }

    async fn request_config_item(&self, item: u8, hash: u16) {
        let mut data = Vec::default();

        data.push(HEADER);
        data.push(self.rib_id);
        data.push(((hash >> 8) & 0xFF) as u8); // LAST_HASH_LOW
        data.push((hash & 0xFF) as u8); // LAST_HASH_HIGH
        data.push(item);
        data.push(0x00);
        data.push(0x00);

        println!("Request config {}", item);

        self.udp_server_send_client
            .lock()
            .await
            .send(&data)
            .await
            .unwrap();
    }

    async fn parse_server_message(&self, device_hash: &mut u16) {
        let data = self.server_data.lock().await;
        let mut new_data = self.new_data.lock().await;

        if !(*new_data) {
            return;
        }

        *new_data = false;

        // if the first bytes isn't the HEADER identifier or if the message isn't inteded for this ID
        if HEADER != data[ServerMessageOffsets::Header as usize]
            || self.rib_id != data[ServerMessageOffsets::RibId as usize]
        {
            println!("Didn't match exptected");
        }

        *device_hash = BigEndian::read_u16(
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
                println!("Recievced hostport config");
                if message_size != 2 {
                    return;
                }
                let host_port = BigEndian::read_u16(
                    &data[(ServerMessageOffsets::PayloadStart as usize)
                        ..=(ServerMessageOffsets::PayloadStart as usize + 1)],
                );
                *self.udp_ports.udp_lin_host_port.lock().await = host_port;
                *self.hashes.host_port_hash.lock().await = *device_hash;
            }
            CLIENT_PORT => {
                println!("Recievced clientport config");
                if message_size != 2 {
                    return;
                }
                let client_port = BigEndian::read_u16(
                    &data[(ServerMessageOffsets::PayloadStart as usize)
                        ..=(ServerMessageOffsets::PayloadStart as usize + 1)],
                );
                *self.udp_ports.udp_lin_client_port.lock().await = client_port;
                *self.hashes.client_port_hash.lock().await = *device_hash;
            }
            MESSAGE_SIZES => {
                println!("Recievced message_sizes config");

                let record_entries = message_size / 3;

                println!("Record entries {}", record_entries);

                let mut count = ServerMessageOffsets::PayloadStart as usize;

                for _s in 0..record_entries as usize {
                    let id = data[count];
                    count += 1;
                    let size = data[count];
                    count += 1;
                    let master = data[count];
                    count += 1;

                    println!("id: {}, size: {}, master: {}", id, size, master);
                }

                *self.hashes.message_sizes_hash.lock().await = *device_hash;
            }
            NODE_MODE => {
                println!("Recievced node_mode config");
                if message_size != 1 {
                    return;
                }
                let node_mode = data[(ServerMessageOffsets::PayloadStart) as usize];
                println!("Nodemode {}", node_mode);
                *self.hashes.node_mode_hash.lock().await = *device_hash;
            }
            NAD => {
                println!("Recievced nad config");
                if message_size != 1 {
                    return;
                }
                let nad = data[(ServerMessageOffsets::PayloadStart) as usize];
                println!("NAD: {}", nad);
                *self.hashes.nad_hash.lock().await = *device_hash;
            }
            _ => {}
        }
    }
}
