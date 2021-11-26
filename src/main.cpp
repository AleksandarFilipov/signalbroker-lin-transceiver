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

/**
 * @author Alvaro Alonso & Niclas Lind
 * @note This project is based on a project from Aleksandar Filipov
 *          https://github.com/volvo-cars/signalbroker-lin-transceiver
 * 
 * @version 2.0.0
 * */

#include <Arduino.h>

#include "Config.hpp"
#include "Records.hpp"
#include "EthernetClient.hpp"
#include "LinUdpGateway.hpp"

constexpr uint8_t rib_id = 8;

EthernetClient ethClient{};
Records records{};
Config config{rib_id, records};
LinUdpGateway linUdpGateway{Serial1, config, records};

void setup()
{
    Serial.begin(115200);
    Serial1.begin(19200);
    ethClient.connect(&config);

    // Init configuration
    config.init();
}

void loop()
{
    // Get configuration from server and send heartbeat
    config.run();

    if (!linUdpGateway.connected())
    {
        if (config.getLockIpAddress())
            linUdpGateway.init();
    }
    else
    {
        linUdpGateway.run();
    }
}