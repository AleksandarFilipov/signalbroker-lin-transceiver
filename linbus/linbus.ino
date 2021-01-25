// Copyright 2019 Volvo Cars
//
// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.  See the NOTICE file
// distributed with this work for additional information
// regarding copyright ownership.  The ASF licenses this file
// to you under the Apache License, Version 2.0 (the
// ”License"); you may not use this file except in compliance
// with the License.  You may obtain a copy of the License at
//
//  http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing,
// software distributed under the License is distributed on an
// “AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied.  See the License for the
// specific language governing permissions and limitations
// under the License.

//__AVR_ATmega32U4__
#ifdef ARDUINO_AVR_MICRO
//#define NIC_ENC28J60
//#define NIC_W5500
#define NIC_W5100

//#define SAVE_MEM

#elif defined ARDUINO_AVR_LEONARDO_ETH
#define NIC_W5500

#elif defined ARDUINO_AVR_FEATHER32U4
#define NIC_W5500

#else
#error "Don't know what i'm building for.."
#endif

#define DEFAULT_LIN_TIMEOUT 1000
// on lin 14, 3 is fine. 2 results in some timeout
// #define TRAFFIC_TIMEOUT 2
// #define TRAFFIC_TIMEOUT 3
#define TRAFFIC_TIMEOUT 4
#define QUERY_SIGNAL_SERVER_TIMEOUT 0
#define QUERY_CONFIG_TIMEOUT 1000


#define MAC 0x00, 0xA7, 0x0A, 0x12, 0x1B, 0x00  //00A70A is VOLVOO upside down with some good will. 12 1B is RIB (obviously..)

// dont turn this on unless you know what you are doing.

#include <HardwareSerial.h>
#include <EEPROM.h>

// https://github.com/zapta/linbus/tree/master/analyzer/arduino
// #include "lin_frame.h"
#include "lin.h"

//#include <SPI.h>         // needed for Arduino versions later than 0018

#ifdef NIC_ENC28J60
#include <UIPEthernet.h>
#include <UIPUdp.h>
#define SAVE_MEM   //The ENC28J60 lacks a TCP/IP-stack! This needs to run on the Arduino and eats a lot of memory. Beware!

#elif defined NIC_W5500
#include <Ethernet.h>
#include <EthernetUdp.h>         // UDP library from: bjoern@cs.stanford.edu 12/30/2008

#elif defined NIC_W5100
#include <Ethernet.h>
#include <EthernetUdp.h>
#endif


#define USE_DHCP true
//If DHCP is NOT used the RIB will take as its IP the ip_base + its ribID.
IPAddress ip_base = {192,168,5,50};
//Until the server adress is known we broadcast on the entire subnet
IPAddress ipserver = {255,255,255,255};


// this is used by the master to send lin BREAK
#define BREAK 0x00
#define SYN_FIELD 0x55
#define RED_PIN 7
#define GREEN_PIN 8
#define BLUE_PIN 9

byte mac[] = {
  MAC
};

#ifndef SAVE_MEM
//#define debugPrintln(...) Udp.beginPacket(ipserver, debugPort); Udp.print("["); Udp.print(mac[5]); Udp.print("]"); Udp.println(__VA_ARGS__); Udp.endPacket();
char printBuffer[80];
#define debugPrintln(...) { snprintf(printBuffer, sizeof(printBuffer), __VA_ARGS__); config.send_logg_to_server(printBuffer); Serial.print(printBuffer); Serial.print("\r\n"); }
#else
#define debugPrintln(...)
#endif

#define UNUSED(expr) { do { (void)(expr); } while (0); }

void(* SWReset) (void) = 0; //declare reset function @ address 0

union DoubleByte
{
    uint16_t u16;
    struct U8 {
      uint8_t low;
      uint8_t high;
    } u8;
};

struct Config {
  #define CONFIG_PORT 4000
  #define CONFIG_PORT_RECEIVE 4001
  #define DEBUG_PORT_BASE 3000
  #define HEARTBEAT_PERIOD 3000 //ms

  // mini protocol....
  // payload might be empty if payload_size is 0
  //   header::8, rib_id::8, hash::16 identifier::8, payload_size::16, payload::payload_size*bytes

  #define HEADER 0x04

  #define HOST_PORT (1<<0)      //1
  #define CLIENT_PORT (1<<1)    //2
  #define MESSAGE_SIZES (1<<2)  //4
  #define NODE_MODE (1<<3)      //8
    #define MODE_MASTER 1
    #define MODE_SLAVE 0
    #define MODE_NONE 0xFF
  #define REASSIGN_RIBID (1<<5) //32
  #define HEART_BEAT (1<<4)
    #define HEART_BEAT_TX_LIN (1)
    #define HEART_BEAT_RX_LIN (2)
    #define HEART_BEAT_TX_UDP (3)
    #define HEART_BEAT_RX_UDP (4)
    #define HEART_BEAT_SYNC_COUNT (5)
    #define HEART_BEAT_UNSYNCHED_PACKAGES (6)
    #define HEART_BEAT_SYNCHED_PACKAGES (7)

  #define LOGGER 0x60                  //96

  #define HEADER_OFFSET 0
  #define RIB_ID_OFFSET 1
  #define HASH_HIGH_OFFSET 2
  #define HASH_LOW_OFFSET 3
  #define IDENTIFIER_OFFSET 4
  #define PAYLOAD_SIZE_HIGH_OFFSET 5
  #define PAYLOAD_SIZE_LOW_OFFSET 6
  #define PAYLOAD_START_OFFSET 7

  void Init() {
//    ribID = getRibID();
    ribID = 1;
    mac[5] = ribID;

    lastHash.u16 = 0xFFFF;
    clientPortHash.u16 = 0;
    hostPortHash.u16 = 0;
    nodeModeHash.u16 = 0;
    messageSizesHash.u16 = 0;

    lastHeartbeat = 0;
  }

  void send_heartbeat();
  void parse_server_message();
  void verify_config();
  void request_config_item(byte item);
#ifndef SAVE_MEM
  void send_logg_to_server(const char* message);
#else
#define send_logg_to_server(X)
#endif
  uint8_t getRibID();

  uint8_t ribID; //This is calculated locally and part of the message header

  uint16_t hostPort;
  DoubleByte hostPortHash;
  uint16_t clientPort;
  DoubleByte clientPortHash;
  //size_t messageSizes;
  DoubleByte messageSizesHash;
  // "MODE_MASTER means that you need to reply to all ids, all except the master ids. For master id the data should be sent back.
  // "MODE_SLAVE" means that we send arbitration frames and master frames. Other responses should be sent back.
  uint8_t nodeMode;
  DoubleByte nodeModeHash;

  DoubleByte lastHash;  //Last server message had this config version (or config hash) and all config fields should match.
  unsigned long lastHeartbeat;
} config;

#define MAX_RECORD_ENTRIES 20
struct Record
{
    byte id;
    byte size;
    byte master;
    byte write_cache[8];
    boolean cache_valid;
} record_list[MAX_RECORD_ENTRIES];

int record_entries;

unsigned int debugPort = DEBUG_PORT_BASE;

// buffers for receiving and sending data
#define UDP_TX_PACKET_MAX_SIZE_CUSTOM 128
byte packetBuffer[UDP_TX_PACKET_MAX_SIZE_CUSTOM];  //buffer to hold incoming packet,

// An EthernetUDP instance to let us send and receive packets over UDP
EthernetUDP Udp;
EthernetUDP UdpConfig;
EthernetUDP UdpConfigReceive;

// byte b, i, n, b2;
// LinFrame frame;
Lin lin(SERIAL_PORT_HARDWARE);

void Abort() {
  pinMode(LED_BUILTIN, OUTPUT);
  for (int i=0; i<10; ++i) {
    digitalWrite(LED_BUILTIN, HIGH);
    delay(100);
    digitalWrite(LED_BUILTIN, LOW);
    delay(100);
  }
  SWReset();
}

int TxOverLin = 0;
int RxOverLin = 0;
int TxOverUdp = 0;
int RxOverUdp = 0;
int global_sync_count = 0;
int global_unsynched_packages = 0;
int global_synched_packages = 0;

void setup() {
  Serial.begin(19200);
  config.Init();

  // configure ethernet
  if (!USE_DHCP || !Ethernet.begin(mac)) { //try DHCP, but if it doesn't work, use a static IP
     ip_base[3] += config.ribID;
     Ethernet.begin(mac, ip_base); //Static IP
  }

  pinMode(LED_BUILTIN, OUTPUT);
  pinMode(RED_PIN, OUTPUT);
  pinMode(GREEN_PIN, OUTPUT);
  pinMode(BLUE_PIN, OUTPUT);

  UdpConfig.begin(CONFIG_PORT);
  UdpConfigReceive.begin(CONFIG_PORT_RECEIVE);

  debugPrintln("LIN Debugging begins");
  lin.begin(19200);
  lin.serial.setTimeout(DEFAULT_LIN_TIMEOUT);
}

void led_status_ok() {
  digitalWrite(LED_BUILTIN, HIGH);

  // external led
  digitalWrite(GREEN_PIN, LOW);
  digitalWrite(RED_PIN, LOW);
  digitalWrite(BLUE_PIN, LOW);
  if (MODE_MASTER == config.nodeMode) {
    digitalWrite(GREEN_PIN, HIGH);
  } else if (MODE_SLAVE == config.nodeMode){
    digitalWrite(BLUE_PIN, HIGH);
  }
}

void red() {
  digitalWrite(LED_BUILTIN, LOW);

  // external led
  digitalWrite(GREEN_PIN, LOW);
  digitalWrite(RED_PIN, LOW);
  digitalWrite(RED_PIN, HIGH);
  digitalWrite(BLUE_PIN, LOW);
}

void flush_udp_buffer(EthernetUDP* flushUdp) {
  int packetSize = 0;
  do {
    packetSize = flushUdp->parsePacket();
  } while (packetSize != 0);
}

// psudo code for simulating slave. Master sends arbritration messages.
// we respond appropriate. This means that slaves must be disconnected
// to avoid bus craziness.
// example frames..
//  3    D0 8 C0 B  6  0  8 BC   8D
// 47    BB F BB F 80 BB 6F   77 (where 77 is checksum and 47 is id)

#ifndef NO_LIN
Record* getRecord(byte id) {
  Record* record = NULL;
  for (int i = 0; i< record_entries; i++) {
    if (record_list[i].id == id) {
      record = &record_list[i];
      break;
    }
  }
  return record;
}

byte readbuffer[10];

void readLinAndSendOnUdp(byte id) {
  // readbuffer   0  1  2        3        4       10
  //              id payload1 payload2...         crc
  // count        0        1        2
  boolean crc_valid = false;
  byte payload_size = 0;


  lin.serial.setTimeout(TRAFFIC_TIMEOUT);
  readbuffer[0] = (byte)(lin.addrParity(id) | id);

  Record* record = getRecord(id);
  if (record != NULL) {
    int verify = 0;
    payload_size = record->size;
    // adding one for crc.
    verify = lin.serial.readBytes(&readbuffer[1], payload_size + 1);
    if (verify == (payload_size + 1)) {
      if ((id == 60) || (id == 61)) {
        // calculate checksum in traditional way for diagnstic frames.
        crc_valid = lin.validateChecksum(&readbuffer[1], payload_size+1);
      } else {
        crc_valid = lin.validateChecksum(&readbuffer[0], payload_size+2);
      }
      RxOverLin++;
    } else {
      // debugPrintln("timeout8 length missmatch");
      // debugPrintln(payload_size + 1);
    }
  } else {
    red();
    //debugPrintln("invalid id: 0x%x (%d)", id, id);
  }
  // return to default timeout.
  lin.serial.setTimeout(DEFAULT_LIN_TIMEOUT);

  if (crc_valid) {
    led_status_ok();
#ifdef VERBOSE_DBG
    debugPrintln("++crc match for id: 0x%x  length is: %d", id, payload_size);
    for(int i = 0; i < payload_size+1; i++)
    {
      debugPrintln("0x%x", readbuffer[i]);
    }
#endif
    sendOverUdp(id, &readbuffer[1], payload_size);
  } else {
    // red();
#ifdef VERBOSE_DBG
    debugPrintln("--crc failed for id: 0x%x  count is: %d", id, payload_size);
#endif
  }
}

byte toServerPayload[] = {
  0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00
};

void sendOverUdp(byte id, byte* payload, byte size) {
  byte* payloadLocal = &toServerPayload[5];
  toServerPayload[3] = id;
  toServerPayload[4] = size;
  for (int i = 0; i < size; i++) {
    payloadLocal[i] = payload[i];
  }
  if (!Udp.beginPacket(ipserver, config.hostPort)) {
    debugPrintln("!!! Failed to open UDP socket: %d, %d, %d, %d: %d for send", ipserver[0], ipserver[1], ipserver[2], ipserver[3], config.hostPort);
  } else {
    Udp.write(toServerPayload, 5+size);
    Udp.endPacket();
    TxOverUdp++;
    //debugPrintln("Sending %d bytes on port %d", size, config.hostPort);
  }
}

// for testing on lin18 frame 0x09
byte hardcoded[] = {
0x01, 0x02, 0x03,
  0x04, 0x2b
};

byte sendbuffer[10];
void sendOverSerial(Record* record) {
  byte id = record->id;
  sendbuffer[0] = lin.addrParity(id) | id;
  byte* sendbuffer_payload = &sendbuffer[1];
  for (int i = 0; i < record->size; i++) {
    sendbuffer_payload[i] = record->write_cache[i];
  }
  byte* crc = &sendbuffer[1+record->size];
  if ((id == 60) || (id == 61)) {
    // calculate checksum in traditional way for diagnstic frames.
    crc[0] = (byte)lin.calulateChecksum(&sendbuffer[1], record->size);
  } else {
    crc[0] = (byte)lin.calulateChecksum(sendbuffer, record->size + 1);
  }
  //this code fixes consequent synch isseus.
  lin.serial.write(&sendbuffer[1], record->size + 1);
  lin.serial.flush();
  lin.serial.end();
  TxOverLin++;
  lin.serial.begin(19200);
}

// byte hardcoded[] = {
//   0x0, 0x0, 0x0, 0x10, 0x08, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08
// };

// {"command":"write","signals":{"CmptmRelHum":27.0, "CmptmtRelHumQf":2,"SnsrStsForBattU":3, "SnsrStsForRelHum":4, "SnsrStsForWindT":5,"SnsrStsForWindTRef":6, "WindDewT":7,"WindDewTQf":8, "WindT":9, "WindTQf":10, "WindTRef":42.5, "WindTRefQf":12, "ErrRespHUS":13}, "namespace" :"Lin"}
// byte hardcoded[] = {
//    0x0, 0x0, 0x0, 0x10, 0x08, 129, 214, 2, 28, 75, 57, 193, 234
// };

byte consumeUdpMessage(byte* buffer, int temp_timeout) {
  int packetSize = 0;
  int bytes = 0;
  unsigned long endtime = millis() + temp_timeout;
  do {
    packetSize = Udp.parsePacket();
    if (packetSize) {
      Udp.setTimeout(temp_timeout);
      bytes = Udp.read(buffer, UDP_TX_PACKET_MAX_SIZE_CUSTOM);
      Udp.setTimeout(DEFAULT_LIN_TIMEOUT);
      // debugPrintln("Read %d on port %d", bytes, config.clientPort);
      RxOverUdp++;
    }
  } while ((packetSize == 0) && (millis() < endtime));
  return bytes;
}


void cacheUdpMessage(Record* record, byte id) {
  int bytes;
  do {
    bytes = consumeUdpMessage(packetBuffer, QUERY_SIGNAL_SERVER_TIMEOUT);
    // fake packetbuffer
    // int bytes = 13;
    // for (int i = 0; i < 13; i++) {
    //   packetBuffer[i] = hardcoded[i];
    // }
    // id plus size shold aways be at least 5 bytes.
    if (bytes >= 4) {
      Record* updateRecord = record;
      if (packetBuffer[3] != id) {
        updateRecord = getRecord(packetBuffer[3]);
        // debugPrintln("missmatch udp responce");
      }
      if (updateRecord == NULL) {
        debugPrintln("missmatch id does not exist");
      } else {
        memcpy(updateRecord->write_cache, &packetBuffer[5], updateRecord->size);
        updateRecord->cache_valid = true;
      }
    } else {
      if ((bytes < 4) && (bytes != 0)) {
        debugPrintln("signalserver payload to short");
        red();
      }
    }
  } while (bytes != 0);
}

void measureSignalBrokerBouce() {
  for (;;) {
    // debugPrintln("staring");
    unsigned long t = micros();
    byte empty = 0;
    sendOverUdp(0x24, &empty, 0);
    int packetSize;
    int bytes;
    UNUSED(bytes);

    //wait for signalserver to respond
    do {
      packetSize = Udp.parsePacket();
      if (packetSize) {
        bytes = Udp.read(packetBuffer, UDP_TX_PACKET_MAX_SIZE_CUSTOM);
      }
    } while (packetSize == 0);
    // debugPrintln("done micros passed: ");
    debugPrintln("measureSignalBrokerBouce: %d", (micros() - t));
    // debugPrintln(" bytes: ");
    // debugPrintln(bytes);
  }
}
#endif //NO_LIN

uint8_t Config::getRibID() {
  //address @0000-@0001 = 0x131B if an ID has been stored in EEPROM
  if (0x13 == EEPROM.read(0) && 0x1B == EEPROM.read(1)) {
    return EEPROM.read(2); //next byte is the RibID
  }

  // Read jumper configuration
  // Pull any combination of pins 2,3 and 4 to LOW to set a RIB ID
  // Pins will initially have random values unless pulled to HIGH or LOW.
  // First pull the config pins to HIGH from SW and then check what pins have been pulled LOW by bridging to GND.
  int jumperCfg = 1; //one-based index
  const int firstCfgPin = 2;
  const int lastCfgPin = 4;
  for (int pin=firstCfgPin; pin<=lastCfgPin; ++pin) {
    pinMode(pin, OUTPUT);
    digitalWrite(pin,HIGH);
    pinMode(pin, INPUT);
    if (LOW == digitalRead(pin)) {
      jumperCfg += 1 << (pin-firstCfgPin);
    }
  }
  return jumperCfg;
}

void Config::send_heartbeat() {

  if (millis() > lastHeartbeat + HEARTBEAT_PERIOD) {
    //flush_udp_buffer(&UdpConfig);
    debugPrintln("Sending heartbeat with hash 0x%x", lastHash.u16);
    UdpConfig.beginPacket(ipserver, CONFIG_PORT);
    UdpConfig.write(HEADER);
    UdpConfig.write(ribID);
    UdpConfig.write(lastHash.u8.high);
    UdpConfig.write(lastHash.u8.low);
    UdpConfig.write(HEART_BEAT);

    //payload size
    UdpConfig.write((byte)0x00);
    UdpConfig.write((byte)21);

    DoubleByte local;

    local.u16 = RxOverLin;
    UdpConfig.write((byte)HEART_BEAT_RX_LIN);
    UdpConfig.write(local.u8.high);
    UdpConfig.write(local.u8.low);

    local.u16 = TxOverLin;
    UdpConfig.write((byte)HEART_BEAT_TX_LIN);
    UdpConfig.write(local.u8.high);
    UdpConfig.write(local.u8.low);

    local.u16 = RxOverUdp;
    UdpConfig.write((byte)HEART_BEAT_RX_UDP);
    UdpConfig.write(local.u8.high);
    UdpConfig.write(local.u8.low);

    local.u16 = TxOverUdp;
    UdpConfig.write((byte)HEART_BEAT_TX_UDP);
    UdpConfig.write(local.u8.high);
    UdpConfig.write(local.u8.low);

//sync
    local.u16 = global_sync_count;
    UdpConfig.write((byte)HEART_BEAT_SYNC_COUNT);
    UdpConfig.write(local.u8.high);
    UdpConfig.write(local.u8.low);

    local.u16 = global_unsynched_packages;
    UdpConfig.write((byte)HEART_BEAT_UNSYNCHED_PACKAGES);
    UdpConfig.write(local.u8.high);
    UdpConfig.write(local.u8.low);

    local.u16 = global_synched_packages;
    UdpConfig.write((byte)HEART_BEAT_SYNCHED_PACKAGES);
    UdpConfig.write(local.u8.high);
    UdpConfig.write(local.u8.low);

//end


    UdpConfig.endPacket();

    lastHeartbeat = millis();

    // debugPrintln("RxOverLin: %d, TxOverLin: %d, RxOverUdp: %d, TxOverUdp: %d",
    //   RxOverLin,TxOverLin,RxOverUdp,TxOverUdp);
    RxOverLin=TxOverLin=RxOverUdp=TxOverUdp=0;
    global_sync_count=global_unsynched_packages=global_synched_packages=0;

  }
}

void Config::request_config_item(byte item) {

  //flush_udp_buffer(&UdpConfig);
  debugPrintln("Rerequesting config item 0x%x", item);
  UdpConfig.beginPacket(ipserver, CONFIG_PORT);
  UdpConfig.write(HEADER);
  UdpConfig.write(ribID);
  UdpConfig.write(lastHash.u8.high);
  UdpConfig.write(lastHash.u8.low);
  UdpConfig.write(item);
  UdpConfig.write((byte)0x00);
  UdpConfig.write((byte)0x00);
  UdpConfig.endPacket();
}

void Config::parse_server_message() {

  int packetSize = UdpConfigReceive.parsePacket();
  if (0 == packetSize) return;

  UdpConfigReceive.read(packetBuffer, UDP_TX_PACKET_MAX_SIZE_CUSTOM);

  if (HEADER != packetBuffer[HEADER_OFFSET]) {
    debugPrintln("Wrong header: %x, should be %x", packetBuffer[HEADER_OFFSET], HEADER);
    return;
  }
  if (ribID != packetBuffer[RIB_ID_OFFSET]) {
    //This packet was intended for another RIB. If broadcasting is used that is not an error, but we are done.
    return;
  }

  //If we are in broadcast mode we can switch to unicast since we now know that the sender is the server.
  if (255 == ipserver[0]) {
    ipserver = UdpConfigReceive.remoteIP();
  }

  lastHash.u8.high = packetBuffer[HASH_HIGH_OFFSET];
  lastHash.u8.low = packetBuffer[HASH_LOW_OFFSET];

  if (0 == packetBuffer[IDENTIFIER_OFFSET]) {
    debugPrintln("Recieved heartbet reply");
    return;
  }

  int message_size = (packetBuffer[PAYLOAD_SIZE_HIGH_OFFSET]<<8)
                   | packetBuffer[PAYLOAD_SIZE_LOW_OFFSET];

  if ((packetSize - PAYLOAD_START_OFFSET) != message_size) {
    if (message_size > (UDP_TX_PACKET_MAX_SIZE_CUSTOM - PAYLOAD_START_OFFSET)) {
      debugPrintln("ERROR UDP_TX_PACKET_MAX_SIZE_CUSTOM %d is smaller than message size %d. Increase constant in code", UDP_TX_PACKET_MAX_SIZE_CUSTOM, message_size);
    } else {
      debugPrintln("payload sizes missmatch: payload is: %d, header says: %d", packetSize-PAYLOAD_START_OFFSET, message_size);
    }
    return;
  }

  switch (packetBuffer[IDENTIFIER_OFFSET]) {
    case HOST_PORT:
      if (2 != message_size) {
        debugPrintln("!!! host port expected message size 2");
        return;
      }
      hostPort = (packetBuffer[PAYLOAD_START_OFFSET]<<8) + packetBuffer[PAYLOAD_START_OFFSET+1];
      hostPortHash = lastHash;
      debugPrintln("Recieved new hostPort: %d", hostPort);
      break;

    case CLIENT_PORT:
      if (2 != message_size) {
        debugPrintln("!!! client port expected message size 2");
        return;
      }
      clientPort = (packetBuffer[PAYLOAD_START_OFFSET]<<8) + packetBuffer[PAYLOAD_START_OFFSET+1];
      Udp.stop();
      if (!Udp.begin(clientPort)) {
        debugPrintln("!!! Failed to open UDP socket on port: %d for listening", clientPort);
        return;
      }
      clientPortHash = lastHash;
      debugPrintln("Recieved new clientPort: %d", clientPort);
      break;

    case MESSAGE_SIZES:
      {
        record_entries = (message_size)/3;
        if (record_entries > MAX_RECORD_ENTRIES) {
          debugPrintln("!!! Recieved more record entries than storage allows! (%d vs %d)",
            record_entries, MAX_RECORD_ENTRIES);
          return;
        }
        int count = PAYLOAD_START_OFFSET;
        for (int i = 0; i < record_entries; i++) {
          record_list[i].id = packetBuffer[count++];
          record_list[i].size = packetBuffer[count++];
          record_list[i].master = packetBuffer[count++];
          record_list[i].cache_valid = false;
        }
      }
      messageSizesHash = lastHash;
      debugPrintln("Recieved new message sizes: %d entries", record_entries);
      break;

    case NODE_MODE:
      if (1 != message_size) {
        debugPrintln("!!! node mode expected message size 1");
        return;
      }
      nodeMode = packetBuffer[PAYLOAD_START_OFFSET];
      nodeModeHash = lastHash;
      debugPrintln("Recieved new node mode: %s", nodeMode==MODE_MASTER ? "MASTER" : "SLAVE");
      break;

    case REASSIGN_RIBID:
      if (1 != message_size) {
        debugPrintln("!!! Reassign RIB ID expected message size 1");
        return;
      }
      EEPROM.write(0, 0x13); //address @0000-@0001 = 0x131B if an ID has been stored in EEPROM
      EEPROM.write(1, 0x1B);
      EEPROM.write(2, packetBuffer[PAYLOAD_START_OFFSET]); //next byte is the RibID
      debugPrintln("Reassigning ribID, now restarting");
      Abort();
      break;

    default:
      debugPrintln("Recieved unrecognized identifier %d (0x%x)", packetBuffer[IDENTIFIER_OFFSET], packetBuffer[IDENTIFIER_OFFSET]);
      break;
  }
}

void Config::verify_config() {

  bool goodConfig = true;
  unsigned long lastUpdate = millis();

  do {
    goodConfig = true;
    if (clientPortHash.u16 != lastHash.u16) {
      goodConfig = false;
      request_config_item(CLIENT_PORT);
    }
    else if (hostPortHash.u16 != lastHash.u16) {
      goodConfig = false;
      request_config_item(HOST_PORT);
    }
    else if (nodeModeHash.u16 != lastHash.u16) {
      goodConfig = false;
      request_config_item(NODE_MODE);
    }
    else if (messageSizesHash.u16 != lastHash.u16) {
      goodConfig = false;
      request_config_item(MESSAGE_SIZES);
    }

    if (!goodConfig) {
      while (millis() < lastUpdate+500) {}
      lastUpdate = millis();
      parse_server_message();
    }
  } while (!goodConfig);
}

#ifndef SAVE_MEM
void Config::send_logg_to_server(const char* message) {
  UdpConfig.beginPacket(ipserver, CONFIG_PORT);
  UdpConfig.write(HEADER);
  UdpConfig.write(config.ribID);
  UdpConfig.write(lastHash.u8.high);
  UdpConfig.write(lastHash.u8.low);
  UdpConfig.write(LOGGER);
  UdpConfig.write((byte)0x00);
  UdpConfig.write((byte)strlen(message));
  UdpConfig.write(message);
  UdpConfig.endPacket();
}
#else
#define send_log_to_server(X)
#endif

////////////////////////////////////////////////////////////////////////////////

///      //  ///  //
///      //  ///  //
///      //  //// //
///      //  // ////
///      //  //  ///
//////// //  //   //

////////////////////////////////////////////////////////////////////////////////

#ifndef NO_LIN

byte synchbuffer[3];
byte synchHeader() {
  int verify = 0;
  boolean match = false;
  int synchCount = 0;
  verify = lin.serial.readBytes(synchbuffer, 3);
  if (verify != 3) {
    red();
    debugPrintln("timeout1");
  }
  match = (synchbuffer[0] == (byte)BREAK) && (synchbuffer[1] == (byte)SYN_FIELD);
#ifdef VERBOSE_DBG
  if (match) debugPrintln("already synched...");
#endif
  if (match) global_synched_packages++;
  // match = (data[0] == (byte)BREAK) && (data[1] == (byte)SYN_FIELD && (data[2] == 0x03));

  while (!match) {
    red();
#ifdef VERBOSE_DBG
    debugPrintln("synching");
#endif
    // align the buffer.
    synchbuffer[0] = synchbuffer[1];
    synchbuffer[1] = synchbuffer[2];
    verify = lin.serial.readBytes(&synchbuffer[2], 1);
    if (verify != 1) {
      red();
      debugPrintln("timeout2");
    }
    // match = (synchbuffer[0] == (byte)BREAK) && (synchbuffer[1] == (byte)SYN_FIELD && (synchbuffer[2] == 0x03));
    match = (synchbuffer[0] == (byte)BREAK) && (synchbuffer[1] == (byte)SYN_FIELD);
    synchCount++;
  }
  if (synchCount > 0) {
    global_sync_count += synchCount;
    global_unsynched_packages++;
    debugPrintln("synch count is %d, synchbuffer[2] & 0x3F = 0x%x", synchCount, synchbuffer[2] & 0x3f);
  }
  led_status_ok();
  return synchbuffer[2];
}

void writeHeader(byte id) {
  byte echo[2];
  lin.serialBreak();       // Generate the low signal that exceeds 1 char.
  lin.serial.write(0x55);  // Sync byte
  lin.serial.write(lin.addrParity(id) | id);  // ID byte
  // lin transiver will echo back, just consume the data.
  lin.serial.readBytes(echo, 2);
}

#endif //NO_LIN

////////////////////////////////////////////////////////////////////////////////

///      ///////  ///////  ///////
///      //   //  //   //  ///   ///
///      //   //  //   //  ///   //
///      //   //  //   //  ///////
///      //   //  //   //  ///
//////// ///////  ///////  ///

////////////////////////////////////////////////////////////////////////////////

void loop() {

/// RIB configuration
  config.send_heartbeat();
  config.parse_server_message();
  config.verify_config();

  led_status_ok();
  // measureSignalBrokerBouce();

  //flush_udp_buffer(&Udp);
  //debugPrintln("udp buffers empty");

/// Actual LIN traffic
#ifndef NO_LIN
  // debugPrintln("Softserial test (this is real serial)");
  if (MODE_MASTER == config.nodeMode) {
    // wait for server/automatic or manually to write arbitration frame
    int bytes = 0;
    bytes = consumeUdpMessage(packetBuffer, QUERY_SIGNAL_SERVER_TIMEOUT);
    if (bytes >= 5) {
      byte id = packetBuffer[3];
      Record* record = getRecord(id);
      boolean validpayload = ((bytes == 5) || (bytes == (5 + record->size)));
      if (validpayload) {
        if (bytes == 5) {
          //this is an arbiration frame
          if (!record->master) {
            writeHeader(id);
            // now consume the result
            readLinAndSendOnUdp(id);
          } else {
            // ignore this - a full frame will arrive on upd. Once it does we write it (length > 5)
          }
        } else {
          // this a master frame. Send it all..
          writeHeader(id);
          memcpy(record->write_cache, &packetBuffer[5], record->size);
          sendOverSerial(record);
        }
      } else {
        debugPrintln("incorrect data received over udp id: 0x%x bytes: 0x%x", id, bytes);
      }
    }
  } else if(MODE_SLAVE == config.nodeMode) {
    if (lin.serial.available()) {
      //find startposition look for BREAK followed BY SYN_FIELD
      // we expect this to be the header if it's not read bytes until conditions are met.
      byte id;
      //we now should have BREAK, SYN_FIELD followed by ID.
      //Now check parity
      //remove parity bits and then add them again. Check if it mathes whats expected
      byte empty = 0;
      byte idbyte = synchHeader();
      id = idbyte & 0x3f;
      if ((lin.addrParity(id) | id) == idbyte) {
        // send this arbitration frame to the server.
        Record *record = getRecord(id);
        // debugPrintln("processing id: ");
        // debugPrintln(id);
        // this works!!
        // if ((id != 0x10) || record->master) {
        if (record->master || (!record->cache_valid)) {
        // if (true) {
        // if (record != NULL && ((boolean)record->master)) {
          // the master is sending data, just consume and report back.
          sendOverUdp(id, &empty, 0);
          // if (record->master) {
            readLinAndSendOnUdp(id);
          // }
        } else {
          // signalserver should response with the expected payload
          if (record->cache_valid) {
            sendOverSerial(record);
            // this is intetionally after writing to serial which might be counterintuitive,
            // in order to meet timing req for short messages
            sendOverUdp(id, &empty, 0);
          }
          // else {
          //   debugPrintln("messed up");
          //   debugPrintln(id);
          // }
        }
        cacheUdpMessage(record, id);
      } else {
        debugPrintln("parity failure - do nothing (id %d)", id);
      }
    }
  } else {
    //We are neither Master nor Slave. ie. unconfigured.
  }

#ifdef VERBOSE_DBG
  debugPrintln("no serial activity");
#endif
#endif NO_LIN
}
