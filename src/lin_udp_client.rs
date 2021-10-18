use byteorder::ReadBytesExt;
use lin_bus::Frame;
use lin_bus::PID;
use serialport::SerialPort;
use signalbroker_lin_transceiver_rp::NodeMode;
use signalbroker_lin_transceiver_rp::BREAK;
use signalbroker_lin_transceiver_rp::SYN_FILED;
use std::error::Error;
use std::io::{Read, Write};
use std::{sync::Arc, time::Duration};
use tokio::{net::UdpSocket, sync::Mutex};
use tracing::{debug, info, warn};

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
    serial: Arc<Mutex<Box<dyn SerialPort>>>,
}

#[repr(u8)]
enum PacketBufferPos {
    Id = 3,
    Size,
    Payload,
}

impl From<PacketBufferPos> for usize {
    fn from(val: PacketBufferPos) -> Self {
        match val {
            PacketBufferPos::Id => 3,
            PacketBufferPos::Size => 4,
            PacketBufferPos::Payload => 5,
        }
    }
}

impl LinUdpClient {
    pub fn new(
        config: Arc<Config>,
        records: Arc<Mutex<Records>>,
        path: &str,
    ) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            config,
            records,
            udp_client_sender: Arc::new(Mutex::new(None)),
            udp_client_listener: Arc::new(Mutex::new(None)),
            data: Arc::new(Mutex::new(Vec::default())),
            new_data: Arc::new(Mutex::new(false)),
            serial: Arc::new(Mutex::new(serialport::new(path, 19_200).open()?)),
        })
    }

    pub async fn run_master(&self) -> Result<(), Box<dyn Error>> {
        let data = self.data.lock().await;
        let mut new_data = self.new_data.lock().await;
        if !*new_data || data.len() < 5 {
            // TODO: return error
            return Ok(());
        }

        *new_data = false;

        let pid = PID::from_id(data[PacketBufferPos::Id as usize]);
        let record;
        {
            let record_list = self.records.lock().await;
            record = record_list.find_by_id_im(pid.get_id());
        }

        let minimum_data_length = 5;

        if let Some(record) = record {
            let valid_payload = (data.len() == minimum_data_length)
                || data.len() == (minimum_data_length + record.size() as usize);

            if !valid_payload {
                println!("Payload not valid");
                // TODO: return error
                return Ok(());
            }

            // If the packet length is equal to 5 and the record isn't a master
            // Then it is an arbitration frame...
            if data.len() == minimum_data_length {
                if !record.is_master() {
                    // Send arbitration message (only the header)
                    self.write_header(pid).await?;
                    // Wait until slave respond and consume the result
                    self.read_lin_and_send_udp(pid).await?;
                }
            } else {
                self.write_header(pid).await?;
                let mut records = self.records.lock().await;

                if let Some(record) = records.find_by_id(pid.get_id()) {
                    record.set_write_cache(
                        &data[PacketBufferPos::Payload.into()
                            ..(PacketBufferPos::Payload as usize + record.size() as usize)],
                    );
                    self.send_over_serial(record).await?;
                }
            }
        };
        Ok(())
    }

    pub async fn send_over_serial(&self, record: &Record) -> Result<(), Box<dyn Error>> {
        let id = record.id();
        let pid = PID::from_id(id);
        let frame = Frame::from_data(pid, record.cache());

        let d = std::time::SystemTime::now();
        let mut serial = self.serial.lock().await;
        serial.write_all(frame.get_data_with_checksum())?;
        // serial
        // .read_exact(&mut [frame.get_data_with_checksum().len() as u8])
        // .unwrap();

        println!(
            "Respond to id={:#04x}, with data={:02?}, took {:?}",
            pid.get_id(),
            frame.get_data_with_checksum(),
            d.elapsed()
        );

        // Read echo from LIN-transceiver
        // let mut echo = vec![0x00u8; frame.get_data_with_checksum().len()];
        // serial.read_exact(&mut echo).unwrap();

        self.config.increment_tx_over_lin().await;
    }

    pub async fn write_header(&self, pid: PID) {
        let mut serial = self.serial.lock().await;

        serial.set_baud_rate(9600).unwrap();
        serial.write_all(&[0x00]).unwrap();
        serial.set_baud_rate(19_200).unwrap();
        serial.write_all(&[0x55, pid.get()]).unwrap();
        serial.flush().unwrap();
        let mut echo = [0; 3];
        serial.read_exact(&mut echo).unwrap_or_default();

        if echo != [0x00, 0x55, pid.get()] {
            println!("Couldn't read echo, read={:#?}", echo);
        }
    }

    /// Read a frame on the LIN-bus and send it over UDP if it was successful
    pub async fn read_lin_and_send_udp(&self, pid: PID) {
        let mut read_buffer = [0; 12];
        read_buffer[0] = pid.get();

        let mut serial = self.serial.lock().await;

        serial.set_timeout(Duration::from_millis(15)).unwrap();

        let record_list = self.records.lock().await;
        let record = record_list.find_by_id_im(pid.get_id());

        if let Some(record) = record {
            let bytes_expected = record.size() as usize + 1;

            let bytes_received = serial
                .read(&mut read_buffer[1..=bytes_expected])
                .unwrap_or_default();

            if bytes_received != bytes_expected {
                warn!(
                    "LinFrameId: {} - Payload doesn't match, bytes_expected={}, bytes_received={}",
                    pid.get_id(),
                    bytes_expected,
                    bytes_received
                );
                return;
            }

            let frame = Frame::from_data(pid, &read_buffer[1..bytes_expected]);

            self.config.increment_rx_over_lin().await;

            if frame.get_checksum() == read_buffer[bytes_expected] {
                self.send_over_udp(frame.get_pid(), frame.get_data_with_checksum())
                    .await;
            }
        }

        serial.set_timeout(Duration::from_secs(1)).unwrap();
    }

    /// Send a message with empty payload on UDP
    async fn send_empty_over_udp(&self, pid: PID) {
        self.send_over_udp(pid, &[]).await;
    }

    /// Send a message with payload over UDP
    async fn send_over_udp(&self, pid: PID, data: &[u8]) {
        let mut to_server = [0; 13];
        let max_udp_payload_length = 11;
        let length = match data.len() {
            0 => 0_u8,
            _ => data.len() as u8 - 1,
        };

        if length > max_udp_payload_length {
            println!(
                "Payload length={} is greater then max={}",
                length, max_udp_payload_length
            );
            return;
        }

        to_server[PacketBufferPos::Id as usize] = pid.get_id();
        to_server[PacketBufferPos::Size as usize] = length;
        if length != 0 {
            // This crash with LIN20
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

    /// Find start position of LIN-Header
    ///
    /// LIN-header looks like this:
    ///
    /// BREAK(0x00) SYN_FIELD(0x55) ID(0xXX)
    ///
    /// Read bytes until conditions are met, return the ID.
    pub async fn synch_header(&self) -> u8 {
        let mut serial_buf: Vec<u8> = vec![0; 3];
        let mut serial = self.serial.lock().await;

        serial.read_exact(&mut serial_buf).unwrap_or_default();

        let mut synch_tries = 0;

        while !(serial_buf[0] == BREAK && serial_buf[1] == SYN_FILED) {
            serial_buf[0] = serial_buf[1];
            serial_buf[1] = serial_buf[2];
            serial_buf[2] = serial.read_u8().unwrap_or(0x00);
            synch_tries = (synch_tries + 1) % std::u16::MAX;
        }

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
            debug!("No new data from broker");
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
        let id = pid & 0b0011_1111;

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
                    self.read_lin_and_send_udp(pid).await;
                } else {
                    self.send_over_serial(&record).await;
                    self.send_empty_over_udp(pid).await;
                }
                self.cache_udp_message(record.id()).await;
            }
        } else {
            warn!("Parity failure {} {}", pid, PID::from_id(id).get());
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
            *udp_sender = Some(UdpSocket::bind("0.0.0.0:0").await.unwrap());
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

        // self.serial.lock().await.flush().unwrap();

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
