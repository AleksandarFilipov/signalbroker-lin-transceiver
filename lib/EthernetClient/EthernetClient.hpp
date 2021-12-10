//
// Created by Niclas Lind on 2020-06-20.
//

#pragma once

#include <IPAddress.h>
#include "ETH.h"
#include "Config.hpp"

class EthernetClient
{
public:
    /**
     * @brief Construct an EthernetClient object
     */
    constexpr EthernetClient()
        : m_ethConnected{true} {};

    /**
    * @brief Connect to Ethernet
    * @param useDHCP - Set flag if you intend to connect with DHCP also fill in all IPAddresses, default is false
    * @param deviceAddress - This device IPAddress
    * @param hostAddress   - The Server address
    * @param subnetAddress - The subnet address
    */
    void connect(Config *config,
                 bool useDHCP = true,
                 const IPAddress &deviceAddress = IPAddress(0, 0, 0, 0),
                 const IPAddress &hostAddress = IPAddress(0, 0, 0, 0),
                 const IPAddress &subnetAddress = IPAddress(0, 0, 0, 0))
    {
        // Wifi/Ethernet Event handler
        WiFi.onEvent([&](WiFiEvent_t event, WiFiEventInfo_t info) {
            const char *TAG = "WifiEvent:";
            switch (event)
            {
            case SYSTEM_EVENT_ETH_START:
            {
                Serial.printf("%s ETH Started\n", TAG);
                String hostname = "LinTransceiver" + String(config->ribID());
                ETH.setHostname(hostname.c_str());
                break;
            }
            case SYSTEM_EVENT_ETH_CONNECTED:
            {
                Serial.printf("%s ETH Connected\n", TAG);
                break;
            }
            case SYSTEM_EVENT_ETH_GOT_IP:
            {
                Serial.printf("%s Received IPAddress\n", TAG);
                config->setDeviceIP(ETH.localIP());
                m_ethConnected = true;
                break;
            }
            case SYSTEM_EVENT_ETH_DISCONNECTED:
            {
                Serial.printf("%s ETH Disconnected\n", TAG);
                m_ethConnected = false;
                break;
            }
            case SYSTEM_EVENT_ETH_STOP:
            {
                Serial.printf("%s ETH Stopped\n", TAG);
                m_ethConnected = false;
                break;
            }
            default:
                break;
            }
        });

        ETH.begin();

        // If DHCP is disable, we need to configure the IP Addresses our self before we continue
        if (!useDHCP)
        {
            ETH.config(deviceAddress, hostAddress, subnetAddress);
        }

        Serial.print("Connecting");
        while (!m_ethConnected)
        {
            Serial.print(".");
            delay(100);
        }
        Serial.println();
    }

    constexpr bool isConnected() const { return m_ethConnected; }

private:
    bool m_ethConnected;
};
