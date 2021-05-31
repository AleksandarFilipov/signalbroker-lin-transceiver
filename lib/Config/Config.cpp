#include "Config.hpp"

#include <Arduino.h>

/**
 * @brief Create a config object
 * @param ribID Your device ribID
 * @param records A storage of records, so you can add records that's received from config
 */
Config::Config(uint8_t ribID, Records &records)
    : m_nad{},
      m_nadHash{},
      m_ribID(ribID),
      m_hostPort{},
      m_hostPortHash{},
      m_clientPort{},
      m_clientPortHash{},
      m_messageSizesHash{},
      m_nodeModeHash{},
      m_packetBufferLength{},
      m_lastHash{0xFFFF},
      m_records{records} {}

/**
 * @brief Init the config-udp
 **/
void Config::init()
{
    // Just start to listen to port
    if (udpClientSender.listen(udpServerConfigPort))
    {
    }

    // Listen to port and handle the incoming message
    if (udpClientReceiver.listen(udpTargetConfigPort))
    {
        udpClientReceiver.onPacket([&](AsyncUDPPacket packet) {
            std::array<char, 100> message{};

            auto remoteIp = packet.remoteIP().toString().c_str();
            auto localIp = packet.localIP();

            sprintf(message.data(), "From: %s: %d, To: %s: %d, Length: %d",
                    remoteIp, packet.remotePort(),
                    localIp.toString().c_str(), packet.localPort(), packet.length());

            m_packetBufferLength = packet.length() > m_packetBuffer.size() ? m_packetBuffer.size() : packet.length();
            memcpy(m_packetBuffer.data(), packet.data(), m_packetBufferLength);

            if (!m_lockIpAddress)
            {
                ipServer = packet.remoteIP();
                m_lockIpAddress = true;
            }
            m_newData = true;
        });
    }
}

/**
 * @brief Sending heartbeat, parse server message & verify config
 * */
void Config::run()
{
    sendHeartbeat();
    parseServerMessage();
    verifyConfig();
}

/**
 * @brief Sending heartbeat to server every X millis sec
 * */
void Config::sendHeartbeat()
{
    for (static ulong lastHeartbeat = 0; millis() > lastHeartbeat + 3000; lastHeartbeat = millis())
    {
        AsyncUDPMessage heartbeat{};
        heartbeat.write(HEADER);
        heartbeat.write(m_ribID);
        heartbeat.write(m_lastHash.u8.high);
        heartbeat.write(m_lastHash.u8.low);
        heartbeat.write(HEART_BEAT);

        //payload size
        heartbeat.write(value(0x00));
        heartbeat.write(value(21));

        DoubleByte local{};

        local.u16 = m_rxOverLin;
        heartbeat.write(value(HeartbeatModes::HEART_BEAT_RX_LIN));
        heartbeat.write(local.u8.high);
        heartbeat.write(local.u8.low);

        local.u16 = m_txOverLin;
        heartbeat.write(value(HeartbeatModes::HEART_BEAT_TX_LIN));
        heartbeat.write(local.u8.high);
        heartbeat.write(local.u8.low);

        local.u16 = m_rxOverUdp;
        heartbeat.write(value(HeartbeatModes::HEART_BEAT_RX_UDP));
        heartbeat.write(local.u8.high);
        heartbeat.write(local.u8.low);

        local.u16 = m_txOverUdp;
        heartbeat.write(value(HeartbeatModes::HEART_BEAT_TX_UDP));
        heartbeat.write(local.u8.high);
        heartbeat.write(local.u8.low);

        //sync
        local.u16 = m_synchCount;
        heartbeat.write(value(HeartbeatModes::HEART_BEAT_SYNC_COUNT));
        heartbeat.write(local.u8.high);
        heartbeat.write(local.u8.low);

        local.u16 = m_unSynchedPackages;
        heartbeat.write(value(HeartbeatModes::HEART_BEAT_UNSYNCHED_PACKAGES));
        heartbeat.write(local.u8.high);
        heartbeat.write(local.u8.low);

        local.u16 = m_synchedPackages;
        heartbeat.write(value(HeartbeatModes::HEART_BEAT_SYNCHED_PACKAGES));
        heartbeat.write(local.u8.high);
        heartbeat.write(local.u8.low);

        udpClientSender.broadcast(heartbeat);

        clearCounters();
    }
}

/**
 * @brief Verifying that the data we got from server seems valid
 * */
void Config::verifyConfig()
{
    bool goodConfig = false;

    while (!goodConfig)
    {
        goodConfig = true;
        if (m_clientPortHash.u16 != m_lastHash.u16)
        {
            goodConfig = false;
            requestConfigItem(CLIENT_PORT);
        }
        else if (m_hostPortHash.u16 != m_lastHash.u16)
        {
            goodConfig = false;
            requestConfigItem(HOST_PORT);
        }
        else if (m_nodeModeHash.u16 != m_lastHash.u16)
        {
            goodConfig = false;
            requestConfigItem(NODE_MODE);
        }
        else if (m_messageSizesHash.u16 != m_lastHash.u16)
        {
            goodConfig = false;
            requestConfigItem(MESSAGE_SIZES);
        }
        else if (m_nadHash.u16 != m_lastHash.u16)
        {
            goodConfig = false;
            requestConfigItem(NAD);
        }
        if (!goodConfig)
        {
            // Wait a little time in between
            delay(200);
            parseServerMessage();
        }
    }
}

/**
 * @breif Requesting item from server
 * @param item: Object we want to have info about
 * */
void Config::requestConfigItem(uint8_t item)
{
    std::array<char, 50> logMessage{};
    sprintf(logMessage.data(), "Requesting config item 0x%x", item);
    Serial.println(logMessage.data());

    AsyncUDPMessage message{};

    message.write(HEADER);
    message.write(m_ribID);
    message.write(m_lastHash.u8.high);
    message.write(m_lastHash.u8.low);
    message.write(item);
    message.write((uint8_t)0x00);
    message.write((uint8_t)0x00);
    udpClientSender.broadcast(message);
}

/**
 * @brief Parse the message coming from the server
 * */
void Config::parseServerMessage()
{
    // We return if we don't have any new data
    if (!m_newData)
    {
        return;
    }

    m_newData = false;

    if (HEADER != m_packetBuffer.at(value(Offsets::HEADER_OFFSET)))
    {
        return;
    }

    if (m_ribID != m_packetBuffer.at(value(Offsets::RIB_ID_OFFSET)))
    {
        // This packet was intended for another RIB. If broadcasting is used that is not an error, but we are done.
        return;
    }

    m_lastHash.u8.high = m_packetBuffer.at(value(Offsets::HASH_HIGH_OFFSET));
    m_lastHash.u8.low = m_packetBuffer.at(value(Offsets::HASH_LOW_OFFSET));

    if (0 == m_packetBuffer.at(value(Offsets::IDENTIFIER_OFFSET)))
    {
        return;
    }

    uint8_t message_size = (m_packetBuffer.at(value(Offsets::PAYLOAD_SIZE_HIGH_OFFSET)) << 8u) |
                           m_packetBuffer.at(value(Offsets::PAYLOAD_SIZE_LOW_OFFSET));

    if ((m_packetBufferLength - value(Offsets::PAYLOAD_START_OFFSET)) != message_size)
    {
        if (message_size > (m_packetBuffer.size() - value(Offsets::PAYLOAD_START_OFFSET)))
        {
            const char *message = "Udp_TX_PACKET_MAX_SIZE_CUSTOM is smaller then message size";
            log(message);
        }
        else
        {
            const char *message = "payload size mismatch";
            log(message);
        }
        return;
    }

    switch (m_packetBuffer.at(value(Offsets::IDENTIFIER_OFFSET)))
    {
    case HOST_PORT:
        if (2 != message_size)
        {
            return;
        }
        m_hostPort = (m_packetBuffer.at(value(Offsets::PAYLOAD_START_OFFSET)) << 8u) +
                     m_packetBuffer.at(value(Offsets::PAYLOAD_START_OFFSET) + 1);
        m_hostPortHash = m_lastHash;
        break;
    case CLIENT_PORT:
        if (2 != message_size)
        {
            return;
        }
        m_clientPort = (m_packetBuffer.at(value(Offsets::PAYLOAD_START_OFFSET)) << 8u) +
                       m_packetBuffer.at(value(Offsets::PAYLOAD_START_OFFSET) + 1);
        m_clientPortHash = m_lastHash;
        break;
    case MESSAGE_SIZES:
    {

        auto record_entries = (message_size) / 3u;
        auto count = value(Offsets::PAYLOAD_START_OFFSET);

        // Before adding new entries, clear the old one.
        m_records.clearRecords();

        for (int i = 0; i < record_entries; i++)
        {
            Record record{};
            int id = m_packetBuffer.at(count++);
            int size = m_packetBuffer.at(count++);
            int master = m_packetBuffer.at(count++);

            record.setId(id);
            record.setSize(size);
            record.setMaster(master);
            record.setCacheValid(false);
            m_records.add(record);
        }

        Serial.println("Records received from server: ");
        for (auto &r : m_records.getRecords())
        {
            Serial.printf("\tId %d size %d master: %d chacevalid: %d\n", r.id(), r.size(), r.master(), r.cacheValid());
        }
    }
        m_messageSizesHash = m_lastHash;
        break;

    case NODE_MODE:
        if (1 != message_size)
        {
            return;
        }
        m_nodeMode = static_cast<NodeModes>(m_packetBuffer.at(value(Offsets::PAYLOAD_START_OFFSET)));
        m_nodeModeHash = m_lastHash;
        break;
    case NAD:
        if (1 != message_size)
        {
            return;
        }
        m_nad = static_cast<uint8_t>(m_packetBuffer.at(value(Offsets::PAYLOAD_START_OFFSET)));
        m_nadHash = m_lastHash;
        break;
    default:
        break;
    }
}

/**
 * @brief Depending on the state of LOG_TO_SERIAL we either log to serialport or udp
 * @param message: The message that will be printed in the log
 * */
void Config::log(const char *message)
{
    if (LOG_TO_SERIAL)
    {
        Serial.println(message);
    }
    else
    {
        logToServer(message);
    }
}

/**
 * @brief Sending log message to Udp-client instead of Serialport
 * @param message: The message that will be printed in the log
 * */
void Config::logToServer(const char *message)
{
    AsyncUDPMessage udpMessage{};
    udpMessage.write(HEADER);
    udpMessage.write(m_ribID);
    udpMessage.write(m_lastHash.u8.high);
    udpMessage.write(m_lastHash.u8.low);
    udpMessage.write(LOGGER);
    udpMessage.write((byte)0x00);
    udpMessage.write((byte)strlen(message));
    udpMessage.println(message);
    udpClientSender.broadcast(udpMessage);
}

/**
 * @brief Clear all counters
 * */
void Config::clearCounters()
{
    m_rxOverLin = 0;
    m_txOverLin = 0;
    m_rxOverUdp = 0;
    m_txOverUdp = 0;
    m_synchCount = 0;
    m_synchedPackages = 0;
    m_unSynchedPackages = 0;
}
