use byteorder::ReadBytesExt;
use lin_bus::{checksum, classic_checksum, PID};
use serialport::SerialPort;
use std::io::{Read, Write};
use std::{sync::Arc, time::Duration};
use tokio::{net::UdpSocket, sync::Mutex};
use tracing::{debug, info, warn};

use crate::config::{Config, NodeMode};
use crate::record::Record;
use crate::records::Records;

pub struct LinUdpClient {
    config: Arc<Config>,
    records: Arc<Mutex<Records>>,
    udp_client_sender: Arc<Mutex<Option<UdpSocket>>>,
    udp_client_listener: Arc<Mutex<Option<UdpSocket>>>,
    data: Arc<Mutex<Vec<u8>>>,
    new_data: Arc<Mutex<bool>>,
    serial: Arc<Mutex<Box<dyn SerialPort>>>,
}

#[repr(u8)]
enum PacketBufferPos {
    Id = 3,
    Size,
    Payload,
}

impl LinUdpClient {
    pub fn new(config: Arc<Config>, records: Arc<Mutex<Records>>, path: &str) -> Self {
        Self {
            config,
            records,
            udp_client_sender: Arc::new(Mutex::new(None)),
            udp_client_listener: Arc::new(Mutex::new(None)),
            data: Arc::new(Mutex::new(Vec::default())),
            new_data: Arc::new(Mutex::new(false)),
            serial: Arc::new(Mutex::new(serialport::new(path, 19_200).open().unwrap())),
        }
    }

    pub async fn run_master(&self) {
        let data = self.data.lock().await;
        let mut new_data = self.new_data.lock().await;

        if !*new_data || data.len() < 5 {
            return;
        }

        *new_data = false;

        let pid = PID::from_id(data[PacketBufferPos::Id as usize]);
        let mut record_list = self.records.lock().await;
        let record = record_list.find_by_id(pid.get_id());

        if record.is_none() {
            return;
        }

        if let Some(record) = record {
            let valid_payload = (data.len() == 5) || data.len() == (5 + record.size() as usize);

            if !valid_payload {
                warn!("Payload not valid");
                return;
            }

            // If the packet length is equal to 5 and the record isn't a master
            // Then it is an arbitration frame...
            if data.len() == 5 {
                if !record.is_master() {
                    // Send arbitration message (only the header)
                    self.write_header(pid).await;
                    // Wait until slave respond and consume the result
                    self.read_lin_and_send_udp(pid).await;
                }
            } else {
                self.write_header(pid).await;
                record.set_write_cache(
                    &data[PacketBufferPos::Payload as usize
                        ..(PacketBufferPos::Payload as usize + record.size() as usize)],
                );
                self.send_over_serial(record).await;
            }
        };
    }

    pub async fn send_over_serial(&self, record: &Record) {
        let mut send_buffer = [0; 12];

        let id = record.id();

        let pid = PID::from_id(id);

        // Add the frame protected ID
        send_buffer[0] = pid.get();
        // Add frame payload
        send_buffer[1..=record.size() as usize].copy_from_slice(record.cache());

        if pid.uses_classic_checksum() {
            send_buffer[record.size() as usize + 1] =
                classic_checksum(&send_buffer[1..=record.size() as usize]);
        } else {
            send_buffer[record.size() as usize + 1] =
                checksum(pid, &send_buffer[1..=record.size() as usize]);
        }

        let mut serial = self.serial.lock().await;

        serial
            .write_all(&send_buffer[1..=record.size() as usize + 1])
            .unwrap();

        serial.flush().unwrap();
        self.config.increment_tx_over_lin().await;
    }

    pub async fn write_header(&self, pid: PID) {
        let mut serial_port = self.serial.lock().await;

        serial_port.set_baud_rate(9600).unwrap();
        serial_port.write_all(&[0x00]).unwrap();
        serial_port.set_baud_rate(19_200).unwrap();
        serial_port.write_all(&[0x55, pid.get()]).unwrap();
        serial_port.flush().unwrap();
        let mut echo = [0; 3];
        serial_port.read_exact(&mut echo).unwrap_or_default();

        if echo != [0x00, 0x55, pid.get()] {
            warn!("Couldn't read echo, read={:#?}", echo);
        }
    }

    pub async fn read_lin_and_send_udp(&self, pid: PID) {
        let mut read_buffer = [0; 12];
        read_buffer[0] = pid.get();
        self.serial
            .lock()
            .await
            .set_timeout(Duration::from_millis(10))
            .unwrap();
        let mut record_list = self.records.lock().await;
        let record = record_list.find_by_id(pid.get_id());
        if record.is_none() {
            return;
        }
        let bytes_expected = record.as_ref().unwrap().size() as usize + 1;

        let bytes_received = self
            .serial
            .lock()
            .await
            .read(&mut read_buffer[1..=bytes_expected])
            .unwrap_or_default();

        if bytes_received != bytes_expected {
            return;
        }

        self.config.increment_rx_over_lin().await;

        let mut crc_valid = true;

        if pid.uses_classic_checksum() {
            let calculated_checksum = classic_checksum(&read_buffer[1..bytes_expected - 1]);
            if calculated_checksum == read_buffer[bytes_expected] {
                crc_valid = true;
            }
        } else {
            let calculated_checksum = checksum(pid, &read_buffer[1..bytes_expected]);
            if calculated_checksum == read_buffer[bytes_expected] {
                crc_valid = true;
            }
        }

        if crc_valid {
            self.send_over_udp(
                pid,
                &read_buffer[1..=bytes_expected],
                record.unwrap().size(),
            )
            .await;
        }

        self.serial
            .lock()
            .await
            .set_timeout(Duration::from_secs(1))
            .unwrap();
    }

    async fn send_empty_over_udp(&self, pid: PID) {
        self.send_over_udp(pid, &[], 0).await;
    }

    async fn send_over_udp(&self, pid: PID, data: &[u8], length: u8) {
        let mut to_server = [0; 13];

        if length > 11 {
            return;
        }

        to_server[PacketBufferPos::Id as usize] = pid.get_id();
        to_server[PacketBufferPos::Size as usize] = length;
        if length != 0 {
            to_server[PacketBufferPos::Payload as usize
                ..=(PacketBufferPos::Payload as usize + length as usize)]
                .copy_from_slice(data);
        }

        let payload_length = length as usize + 5;

        debug!("Sending to server {:#?}", &to_server[..payload_length]);
        self.udp_client_sender
            .lock()
            .await
            .as_ref()
            .unwrap()
            .send(&to_server[..payload_length])
            .await
            .unwrap();
        self.config.increment_tx_over_udp().await;
    }

    pub async fn synch_header(&self) -> u8 {
        let mut serial_buf: Vec<u8> = vec![0; 3];
        let mut serial = self.serial.lock().await;

        serial
            .read_exact(serial_buf.as_mut_slice())
            .unwrap_or_default();

        let mut synch_tries = 0;

        while !(serial_buf[0] == 0x00 && serial_buf[1] == 0x55) {
            serial_buf[0] = serial_buf[1];
            serial_buf[1] = serial_buf[2];
            serial_buf[2] = serial.read_u8().unwrap_or(0x00);
            synch_tries += 1;
        }

        println!("Received header {:#?}", &serial_buf);

        self.config.increment_synched_packages().await;

        if synch_tries > 0 {
            self.config.increment_un_synched_packages().await;
            self.config.set_synch_count(synch_tries).await;
        }

        serial_buf[2]
    }

    pub async fn cache_udp_message(&self, _record_id: u8) {
        let data = self.data.lock().await;
        let mut new_data = self.new_data.lock().await;

        if !*new_data {
            // println!("No new data from broker");
            return;
        }

        *new_data = false;

        if data.len() < 4 && data.len() != 0 {
            println!("Data doesn't match");
            return;
        }

        let id = data[PacketBufferPos::Id as usize];

        let mut records = self.records.lock().await;
        let record = records.find_by_id(id);

        if let Some(record) = record {
            record.set_write_cache(
                &data[PacketBufferPos::Payload as usize
                    ..(PacketBufferPos::Payload as usize + record.size() as usize)],
            );
            record.set_cache_valid(true);
        }
    }

    pub async fn run_slave(&self) {
        let pid = self.synch_header().await;
        let id = pid & 0x3f;

        if PID::from_id(id).get() == pid {
            let record;
            {
                let records = self.records.lock().await;
                record = records.find_by_id_im(id);
            }

            if let Some(record) = record {
                let pid = PID::from_id(id);
                if record.is_master() || !record.cache_valid() {
                    self.send_empty_over_udp(pid).await;
                    // self.read_lin_and_send_udp(pid).await;
                } else if record.cache_valid() {
                    self.send_over_serial(&record).await;
                    self.send_empty_over_udp(pid).await;
                }
                self.cache_udp_message(record.id()).await;
            }
        } else {
            println!("Parity failure {} {} {}", id, pid, PID::from_id(id).get());
        }
    }

    pub async fn run(&self) {
        while !self.config.received_ip().await {
            println!("Waiting for server ip...");
            tokio::time::sleep(Duration::from_millis(3000)).await;
        }

        {
            let udp_socket_ip = self.config.host_ip().await;
            let udp_socket_port = self.config.host_port().await;
            let udp_client_sender_address = format!("{}:{}", udp_socket_ip, udp_socket_port);
            info!("client_sender_address: {}", udp_client_sender_address);
            let mut udp_sender = self.udp_client_sender.lock().await;
            *udp_sender = Some(UdpSocket::bind("[::]:0").await.unwrap());
            udp_sender
                .as_ref()
                .unwrap()
                .connect(&udp_client_sender_address)
                .await
                .unwrap();
        }

        let udp_socket_client_port = self.config.client_port().await;
        let udp_client_listener_address = format!("0.0.0.0:{}", udp_socket_client_port);
        info!("client_listener_address: {}", udp_client_listener_address);
        *self.udp_client_listener.lock().await =
            Some(UdpSocket::bind(&udp_client_listener_address).await.unwrap());
        tokio::spawn({
            let client = self.udp_client_listener.clone();
            let new_data = self.new_data.clone();
            let data = self.data.clone();
            let config = self.config.clone();
            async move {
                loop {
                    let mut buf = vec![0; 128];
                    let len = client
                        .lock()
                        .await
                        .as_ref()
                        .unwrap()
                        .recv(&mut buf)
                        .await
                        .unwrap();
                    *data.lock().await = buf[..len as usize].to_owned();
                    *new_data.lock().await = true;
                    config.increment_rx_over_udp().await;
                    debug!(
                        "Received {} bytes with data {:#?}",
                        len,
                        &buf[..len as usize]
                    );
                }
            }
        });

        loop {
            match self.config.node_mode().await {
                NodeMode::Slave => {
                    self.run_slave().await;
                }
                NodeMode::Master => {
                    self.run_master().await;
                }
                NodeMode::Undefined => {}
            }
        }
    }
}
