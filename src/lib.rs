use std::sync::Arc;

use serde_derive::Deserialize;
use tokio::sync::Mutex;

/// UartConfig
///
/// `lin_ports` helds a collection of LinConfig-objects
#[derive(Debug, Deserialize)]
pub struct UartConfig {
    pub lin_ports: Vec<LinConfig>,
}

/// LinConfig
///  
/// `uart` physical bus on-device (such as /dev/ttyAMA0)
///
/// `rib_id` this id should match the id in beamybroker configuration  
#[derive(Debug, Deserialize)]
pub struct LinConfig {
    pub uart: String,
    pub rib_id: u8,
}

pub const HEADER: u8 = 0x04;
pub const HOST_PORT: u8 = 0x01;
pub const CLIENT_PORT: u8 = 0x02;
pub const MESSAGE_SIZES: u8 = 0x04;
pub const NODE_MODE: u8 = 0x08;
pub const HEART_BEAT: u8 = 0x10;
pub const NAD: u8 = 0x20;

pub const BREAK: u8 = 0x00;
pub const SYN_FILED: u8 = 0x55;

#[repr(u8)]
#[derive(Clone)]
pub enum NodeMode {
    Slave,
    Master,
    Undefined = 0xFF,
}

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

impl From<u8> for NodeMode {
    fn from(val: u8) -> Self {
        match val {
            0x00 => NodeMode::Slave,
            0x01 => NodeMode::Master,
            0xFF => NodeMode::Undefined,
            _ => todo!(),
        }
    }
}

impl From<NodeMode> for u8 {
    fn from(val: NodeMode) -> Self {
        match val {
            NodeMode::Slave => 0x00,
            NodeMode::Master => 0x01,
            NodeMode::Undefined => 0xFF,
        }
    }
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

#[derive(Clone, Debug)]
pub struct Counter {
    pub rx_over_lin: u16,
    pub tx_over_lin: u16,
    pub rx_over_udp: u16,
    pub tx_over_udp: u16,
    pub sync_count: u16,
    pub unsynched_packages: u16,
    pub synched_packages: u16,
}

impl Counter {
    pub fn new() -> Self {
        Self {
            rx_over_lin: 0,
            tx_over_lin: 0,
            rx_over_udp: 0,
            tx_over_udp: 0,
            sync_count: 0,
            unsynched_packages: 0,
            synched_packages: 0,
        }
    }

    pub fn clear_counters(&mut self) {
        self.rx_over_lin = 0;
        self.tx_over_lin = 0;
        self.rx_over_udp = 0;
        self.tx_over_udp = 0;
        self.sync_count = 0;
        self.unsynched_packages = 0;
        self.synched_packages = 0;
    }
}

impl Default for Counter {
    fn default() -> Self {
        Self::new()
    }
}

pub struct ConfigHash {
    pub device_hash: Arc<Mutex<u16>>,
    pub client_port_hash: Arc<Mutex<u16>>,
    pub host_port_hash: Arc<Mutex<u16>>,
    pub node_mode_hash: Arc<Mutex<u16>>,
    pub message_sizes_hash: Arc<Mutex<u16>>,
    pub nad_hash: Arc<Mutex<u16>>,
}

impl ConfigHash {
    pub fn new() -> Self {
        let default_hash = 0xFFFF_u16;

        Self {
            device_hash: Arc::new(Mutex::new(default_hash)),
            client_port_hash: Arc::new(Mutex::new(default_hash)),
            host_port_hash: Arc::new(Mutex::new(default_hash)),
            node_mode_hash: Arc::new(Mutex::new(default_hash)),
            message_sizes_hash: Arc::new(Mutex::new(default_hash)),
            nad_hash: Arc::new(Mutex::new(default_hash)),
        }
    }
}

impl Default for ConfigHash {
    fn default() -> Self {
        Self::new()
    }
}

pub struct UdpPort {
    pub udp_server_config_port: u16,
    pub udp_target_config_port: u16,
    pub udp_lin_host_port: u16,
    pub udp_lin_client_port: u16,
}

impl UdpPort {
    pub fn new() -> Self {
        Self {
            udp_server_config_port: 4001,
            udp_target_config_port: 4000,
            udp_lin_client_port: 0,
            udp_lin_host_port: 0,
        }
    }
}

impl Default for UdpPort {
    fn default() -> Self {
        Self::new()
    }
}
