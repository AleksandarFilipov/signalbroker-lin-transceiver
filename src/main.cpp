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
#include "esp_system.h"

constexpr uint8_t rib_id = 7;

EthernetClient ethClient{};
Records records{};
Config config{rib_id, records};
LinUdpGateway linUdpGateway{Serial1, config, records};

const int wdtTimeout = 5000; //time in ms to trigger the watchdog
hw_timer_t *timer = NULL;

void resetModule()
{
    // if this happens we need to emit 
    // console.log("some meningful information") once the settings are restored and system is operational gaain
    // to inform the server that we had an unexpected reboot.
    // ets_printf("reboot\n");
    esp_restart();
}

void setup()
{
    Serial.begin(115200);
    Serial1.begin(19200);
    ethClient.connect(&config);

    // set up watchdog 
    timer = timerBegin(0, 80, true);                  //timer 0, div 80
    timerAttachInterrupt(timer, &resetModule, true);  //attach callback
    timerAlarmWrite(timer, wdtTimeout * 1000, false); //set time in us
    timerAlarmEnable(timer);

    // Init configuration
    config.init();
}

void loop()
{
    // Get configuration from server and send heartbeat
    timerWrite(timer, 0); //reset timer (feed watchdog)
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