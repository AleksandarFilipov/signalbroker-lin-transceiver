use std::{error::Error, sync::Arc, time::Duration};

use byteorder::{ByteOrder, LittleEndian};
use config::{HeartBeatModes, ServerMessageOffsets};
use tokio::{net::UdpSocket, sync::Mutex, time::sleep};

mod config;

// const HEADER: u8 = 0x04;
const RIB_ID: u8 = 0x01;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let hash = Arc::new(Mutex::new(0xFFFF_u16));
    let client_port_hash = Arc::new(Mutex::new(0xFFFF_u16));
    let host_port_hash = Arc::new(Mutex::new(0xFFFF_u16));
    let node_mode_hash = Arc::new(Mutex::new(0xFFFF_u16));
    let message_sizes_hash = Arc::new(Mutex::new(0xFFFF_u16));
    let nad_hash = Arc::new(Mutex::new(0xFFFF_u16));

    let one = tokio::spawn({
        let hash = hash.clone();

        async move {
            let server_receive = UdpSocket::bind("[::]:4000").await.unwrap();

            const MAX_DATAGRAM_SIZE: usize = 128;
            let mut data = vec![0u8; MAX_DATAGRAM_SIZE];

            // server_receive.connect("127.0.0.1:4000").await.unwrap();
            let len = server_receive.recv(&mut data).await.unwrap();

            println!("{:#?}", &data[..len]);

            parse_server_message(data, &hash).await;
        }
    });

    let two = tokio::spawn({
        let hash = hash.clone();
        async move {
            let server_send = Arc::new(Mutex::new(UdpSocket::bind("[::]:0").await.unwrap()));
            server_send
                .lock()
                .await
                .connect("127.0.0.1:4001")
                .await
                .unwrap();
            server_send.lock().await.set_broadcast(true).unwrap();
            loop {
                tokio::spawn({
                    let client = server_send.clone();
                    let hash = hash.clone();
                    async move { send_heartbeat(&client, &hash).await }
                })
                .await.unwrap();
                // .await
                // .unwrap();
            }
        }
    });

    tokio::join!(one, two).0.unwrap();

    Ok(())
}

async fn parse_server_message(data: Vec<u8>, hash: &Arc<Mutex<u16>>) {
    let mut hash = hash.lock().await;

    // if the first bytes isn't the HEADER identifier or if the message isn't inteded for this ID
    if config::HEADER != data[ServerMessageOffsets::Header as usize]
        || RIB_ID != data[ServerMessageOffsets::RibId as usize]
    {
        println!("Didn't match exptected");
    }

    *hash = LittleEndian::read_u16(
        &data[(ServerMessageOffsets::HashHigh as usize)..=(ServerMessageOffsets::HashLow as usize)],
    );

    println!("{}", hash);
}

async fn verify_config(client: &Arc<Mutex<UdpSocket>>) {
    let mut good_config = false;

    if good_config == true {}
}

async fn send_heartbeat(client: &Arc<Mutex<UdpSocket>>, hash: &Arc<Mutex<u16>>) {
    // let server_send = UdpSocket::bind("[::]:0").await.unwrap();
    // server_send.connect("127.0.0.1:4001").await.unwrap();
    // server_send.set_broadcast(true).unwrap();
    sleep(Duration::from_millis(2500)).await;
    let hash = hash.lock().await;

    let mut data = Vec::default();
    data.push(config::HEADER); // HEADER
    data.push(RIB_ID); // RIB_ID
    data.push((*hash & 0xFF) as u8); // LAST_HASH_HIGH
    data.push(((*hash >> 8) & 0xFF) as u8); // LAST_HASH_LOW
    data.push(config::HEART_BEAT); // HEARTBEAT

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

    client.lock().await.send(&data).await.unwrap();
}
