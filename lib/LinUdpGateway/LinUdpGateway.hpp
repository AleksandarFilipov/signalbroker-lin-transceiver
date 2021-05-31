#pragma once

#include <Arduino.h>
#include <AsyncUDP.h>

#include "lin.hpp"
#include "Common.hpp"

class Config;
class Records;
class Record;

class LinUdpGateway {
    enum class DiagnosticFrameId : uint8_t {
        MasterRequest = 60,
        SlaveResponse = 61
    };

public:
    LinUdpGateway(HardwareSerial &serial, Config &config, Records &records);

    void init();

public:
    void readLinAndSendOnUdp(byte id);

    void writeHeader(byte id);

    uint8_t synchHeader();

    void sendOverUdp(byte id);

    void sendOverUdp(byte id, const byte *payload, byte size);

    void sendOverSerial(Record *record);

    void cacheUdpMessage(Record *record);

    bool connected() { return m_udpClient.connected(); }

    void run();

private:
    void runMaster();

    void runSlave();

private:
    boolean is_request_intened_for_this_slave(int id);
    AsyncUDP m_udpClient;
    AsyncUDP m_udpListen;
    Lin m_lin;
    Config *m_config;
    Records *m_records;
    int m_last_seen_nad;

    std::array<uint8_t, UDP_TX_PACKET_MAX_SIZE_CUSTOM> _packetBuffer{};
    int m_packetBufferLength{};

    static constexpr uint8_t BREAK = 0x00;
    static constexpr uint8_t SYN_FIELD = 0x55;

    // this is used by the master to send lin BREAK
    // on lin 14, 3 is fine. 2 results in some timeout
    static constexpr uint8_t TRAFFIC_TIMEOUT = 10;
    static constexpr uint8_t QUERY_SIGNAL_SERVER_TIMEOUT = 0;
    static constexpr uint16_t DEFAULT_LIN_TIMEOUT = 1000;

    // From signal broker-server we receive UDP-packets ordered in this way
    static constexpr uint8_t PACKET_BUFFER_ID_POS = 3;
    static constexpr uint8_t PACKET_BUFFER_SIZE_POS = 4;
    static constexpr uint8_t PACKET_BUFFER_PAYLOAD_POS = 5;

    bool newData = false;
};