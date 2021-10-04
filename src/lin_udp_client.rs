use std::{sync::Arc, time::Duration};

use tokio::{net::UdpSocket, sync::Mutex};

use crate::config::Config;
use crate::record::Record;
use crate::records::Records;

pub struct LinUdpClient {
    config: Arc<Config>,
    records: Arc<Mutex<Records>>,
    udp_client_sender: Arc<Mutex<Option<UdpSocket>>>,
    udp_client_listener: Arc<Mutex<Option<UdpSocket>>>,
    data: Arc<Mutex<Vec<u8>>>,
    new_data: Arc<Mutex<bool>>,

}

#[repr(u8)]
enum PacketBufferPos {
    Id = 3,
    Size,
    Payload,
}

impl LinUdpClient {
    pub fn new(config: Arc<Config>, records: Arc<Mutex<Records>>) -> Self {
        Self {
            config,
            records,
            udp_client_sender: Arc::new(Mutex::new(None)),
            udp_client_listener: Arc::new(Mutex::new(None)),
            data: Arc::new(Mutex::new(Vec::default())),
            new_data: Arc::new(Mutex::new(false)),
        }
    }

    pub async fn run_master(&self) {
        let data = self.data.lock().await;
        let mut new_data = self.new_data.lock().await;
        let sender;
        {
            sender = self.udp_client_sender.lock().await;
        }

        if !*new_data || data.len() < 5 {
            return;
        }

        *new_data = false;

        let id = data[PacketBufferPos::Id as usize];
        let record = self.records.lock().await.find_by_id(id);

        if record.is_none() {
            return;
        }

        let valid_payload = ((data.len() == 5) || data.len() == (5 + record.as_ref().
            unwrap().size() as usize));

        if !valid_payload {
            return;
        }

        if data.len() == 5 && record.unwrap().master() == 0 {
            self.write_header();
            self.read_lin_and_send_udp();
        } else {
            self.write_header();
            // record.unwrap().set_write_cache(data.as_slice());
        }
        self.send_over_serial().await;
    }

    pub async fn send_over_serial(&self) {
        self.config.increment_rx_over_lin().await;
        // todo!()
    }

    pub fn write_header(&self) {
        // todo!()
    }

    pub fn read_lin_and_send_udp(&self) {
        // todo!()
    }

    pub async fn run(&self) {
        while !self.config.received_ip().await {
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
        {
            let udp_socket_ip = self.config.host_ip().await;
            let udp_socket_port = self.config.host_port().await;
            let udp_client_sender_address = format!("{}:{}", udp_socket_ip, udp_socket_port);
            println!("client_sender_address: {}", udp_client_sender_address);
            let mut udp_sender = self.udp_client_sender.lock().await;
            *udp_sender = Some(UdpSocket::bind("[::]:0").await.unwrap());
            udp_sender.as_ref()
                .unwrap()
                .connect(&udp_client_sender_address)
                .await
                .unwrap();
        }

        let udp_socket_client_port = self.config.client_port().await;
        let udp_client_listener_address = format!("[::]:{}", udp_socket_client_port);
        println!("client_listener_address: {}", udp_client_listener_address);
        *self.udp_client_listener.lock().await = Some(UdpSocket::bind(&udp_client_listener_address).await.unwrap());
        tokio::spawn({
            let client = self.udp_client_listener.clone();
            let new_data = self.new_data.clone();
            let data = self.data.clone();
            async move {
                loop {
                    println!("client={:#?}", client);
                    let mut buf = vec![0; 128];
                    let len = client.lock().await.as_ref().unwrap().recv(&mut buf).await.unwrap();
                    *data.lock().await = buf[..len as usize].to_owned();
                    *new_data.lock().await = true;
                    println!("Received {} bytes with data {:#?}", len, &buf[..len as usize]);
                }
            }
        });

        loop {
            match self.config.node_mode().await {
                0x00 => {
                    // println!("Running slave");
                }
                0x01 => {
                    self.run_master().await;
                }
                _ => {}
            }
        }
    }
}
