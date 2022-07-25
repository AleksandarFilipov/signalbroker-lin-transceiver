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

constexpr uint8_t ledPin = 13; //pin controlling yellow LED
constexpr uint8_t masterPin = 5; //pin setting lin transceiver master or slave. High=master, low=slave
constexpr uint8_t adressPin1 = 32;
constexpr uint8_t adressPin2 = 16;
constexpr uint8_t adressPin3 = 15;
constexpr uint8_t adressPin4 = 14;

EthernetClient ethClient{};
Records records{};
Config *config;
// LinUdpGateway linUdpGateway{Serial1, config, records};
LinUdpGateway *linUdpGateway;

void setup()
{
    Serial.begin(115200);
    Serial1.begin(19200);

    pinMode(ledPin, OUTPUT);
    pinMode(masterPin, OUTPUT);
    pinMode(adressPin1, INPUT);
    pinMode(adressPin2, INPUT);
    pinMode(adressPin3, INPUT);
    pinMode(adressPin4, INPUT);

    pinMode(adressPin1, INPUT_PULLUP);
    pinMode(adressPin2, INPUT_PULLUP);
    pinMode(adressPin3, INPUT_PULLUP);
    pinMode(adressPin4, INPUT_PULLUP);

    digitalWrite(ledPin, HIGH); //turning yellow LED on

    //Calculating adress and printing. Adress is determined by 4 inverted bits.
    uint8_t rib_id = !digitalRead(adressPin1) + (!digitalRead(adressPin2) << 1) +  (!digitalRead(adressPin3) << 2) +  (!digitalRead(adressPin4) << 3);
    // uint8_t rib = 2;
    config = new Config {rib_id, records, masterPin};
    linUdpGateway = new LinUdpGateway{Serial1, *config, records};

    ethClient.connect(config);

    // Init configuration
    config->init();
}

void loop()
{
    // Get configuration from server and send heartbeat
    // Serial.println(rib_id);
    config->run();

    if (!linUdpGateway->connected())
    {
        if (config->getLockIpAddress())
            linUdpGateway->init();
    }
    else
    {
        linUdpGateway->run();
    }
}