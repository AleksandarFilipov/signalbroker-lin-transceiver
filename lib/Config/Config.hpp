#pragma once

#include <Stream.h>
#include <AsyncUDP.h>
#include <array>

#include "Records.hpp"
#include "Common.hpp"

class Config
{
    union DoubleByte
    {
        uint16_t u16;
        struct U8
        {
            uint8_t low;
            uint8_t high;
        } u8;
    };

public:
    explicit Config(uint8_t ribID, Records &records);

    /**
    * "MASTER means that you need to reply to all ids, all except the master ids. For master id the data should be sent back.
    * 
    * "SLAVE" means that we send arbitration frames and master frames. Other responses should be sent back.
    * 
    * "UNDEFINED" means that you haven't chosen any mode yet
    */
    enum class NodeModes : uint8_t
    {
        SLAVE = 0,
        MASTER = 1,
        UNDEFINED = 0xFF
    };

    enum class HeartbeatModes : uint8_t
    {
        HEART_BEAT_TX_LIN = 1,
        HEART_BEAT_RX_LIN = 2,
        HEART_BEAT_TX_UDP = 3,
        HEART_BEAT_RX_UDP = 4,
        HEART_BEAT_SYNC_COUNT = 5,
        HEART_BEAT_UNSYNCHED_PACKAGES = 6,
        HEART_BEAT_SYNCHED_PACKAGES = 7
    };

    enum class Offsets : uint8_t
    {
        HEADER_OFFSET = 0,
        RIB_ID_OFFSET = 1,
        HASH_HIGH_OFFSET = 2,
        HASH_LOW_OFFSET = 3,
        IDENTIFIER_OFFSET = 4,
        PAYLOAD_SIZE_HIGH_OFFSET = 5,
        PAYLOAD_SIZE_LOW_OFFSET = 6,
        PAYLOAD_START_OFFSET = 7
    };

public:
    void init();

    void run();

    void requestConfigItem(uint8_t item);

    void incrementTxOverLin() { m_txOverLin++; }

    void incrementTxOverUdp() { m_txOverUdp++; }

    void incrementRxOverLin() { m_rxOverLin++; }

    void incrementRxOverUdp() { m_rxOverUdp++; }

    void setSynchedCount(int count) { m_synchCount = count; }

    void incrementSynchedPackages() { m_synchedPackages++; }

    void incrementUnSynchedPackages() { m_unSynchedPackages++; }

    IPAddress serverIp() { return ipServer; }

    bool getLockIpAddress() const { return m_lockIpAddress; }

public:
    NodeModes nodeMode() const { return m_nodeMode; }

    uint8_t nad() const { return m_nad; }

    uint8_t ribID() const { return m_ribID; }

    uint16_t hostPort() const { return m_hostPort; }

    uint16_t clientPort() const { return m_clientPort; }

    void log(const char *message);

    void setDeviceIP(const IPAddress &address) { deviceIP = address; };

private:
    void sendHeartbeat();

    void parseServerMessage();

    void verifyConfig();

    void logToServer(const char *message);

    void clearCounters();

private:
    static constexpr auto udpServerConfigPort = 4001;
    static constexpr auto udpTargetConfigPort = 4000;
    static constexpr int heartbeatPeriod_ = 2500;
    static constexpr uint8_t HEADER = 0x04;
    static constexpr uint8_t HOST_PORT = (1u << 0u);     // 1
    static constexpr uint8_t CLIENT_PORT = (1u << 1u);   // 2
    static constexpr uint8_t MESSAGE_SIZES = (1u << 2u); // 4
    static constexpr uint8_t NODE_MODE = (1u << 3u);     // 8
    static constexpr uint8_t HEART_BEAT = (1u << 4u);    // 16
    static constexpr uint8_t NAD = (1u << 5u);           // 32
    static constexpr uint8_t LOGGER = 0x60;              // 96

    uint8_t m_nad = 0;          // This is intended for filtering diagnostic request. To selectively only answer with on lin client.
    DoubleByte m_nadHash;

    uint8_t m_ribID;            // This is calculated locally and part of the message header
    AsyncUDP udpClientSender;   // Udp-client that sends data to signal broker
    AsyncUDP udpClientReceiver; // Udp-client that receives data from the signal broker

    uint16_t m_hostPort; // host port to be used for udp-client in LinUpdGateway
    DoubleByte m_hostPortHash;

    uint16_t m_clientPort;
    DoubleByte m_clientPortHash;

    DoubleByte m_messageSizesHash;

    NodeModes m_nodeMode = NodeModes::UNDEFINED;
    DoubleByte m_nodeModeHash;

    std::array<uint8_t, UDP_TX_PACKET_MAX_SIZE_CUSTOM> m_packetBuffer{};
    int m_packetBufferLength;
    bool m_newData = false;

    int m_txOverLin = 0;
    int m_rxOverLin = 0;
    int m_txOverUdp = 0;
    int m_rxOverUdp = 0;
    int m_synchCount = 0;
    int m_synchedPackages = 0;
    int m_unSynchedPackages = 0;

    // Until the server address is known we broadcast on the entire subnet
    IPAddress ipServer = {255, 255, 255, 255};

    // This ESP32 device IP
    IPAddress deviceIP{};

    // Until we have received an ipaddress from the server, we have this set to false
    bool m_lockIpAddress = false;

    DoubleByte m_lastHash; // Last server message had this config version (or config hash) and all config fields should match.
    Records &m_records;    // In here we store all out records
};
