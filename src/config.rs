use std::net::{IpAddr, Ipv4Addr};

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

struct Config {
    udp_server_config_port: i32,
    udp_target_config_port: i32,
    heart_beat_period: i32,
    ip_address_server: IpAddr,
    // udp_server_send_client: UdpSocket,
    // udp_server_listen_client: UdpSocket,
}

impl Config {
    fn new() -> Self {
        Self {
            udp_server_config_port: 4001,
            udp_target_config_port: 4000,
            heart_beat_period: 2500,
            ip_address_server: IpAddr::V4(Ipv4Addr::new(255, 255, 255, 255)),
            // udp_server_send_client:
        }
    }

    fn send_heartbeat(&self) {}
}
