/*
 Copyright 2011 G. Andrew Stone
All rights reserved.

Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions are met:
    * Redistributions of source code must retain the above copyright
      notice, this list of conditions and the following disclaimer.
    * Neither the name of the copyright holder nor the
      names of its contributors may be used to endorse or promote products
      derived from this software without specific prior written permission.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS" AND
ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED
WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
DISCLAIMED. IN NO EVENT SHALL <COPYRIGHT HOLDER> BE LIABLE FOR ANY
DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES
(INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES;
LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND
ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
(INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS
SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
*/

#include "lin.hpp"

#include <array>

#include "HardwareSerial.h"

void Lin::begin() { m_Serial.begin(serialSpd); }

void Lin::serialBreak() {
    constexpr auto LIN_BREAK_DURATION = 13;
    constexpr auto brakeEnd =
        (1000000UL / static_cast<unsigned long>(serialSpd));
    constexpr auto brakeBegin = brakeEnd * LIN_BREAK_DURATION;
    // GPIO_NUM_4 is the same pin as m_TxPin

    m_Serial.flush();
    // removing this delay results in a master approx 20 min after boot.
    delay(1);
    m_Serial.end();
    // gpio_reset_pin((gpio_num_t)m_TxPin);
    gpio_matrix_out(m_TxPin, SIG_GPIO_OUT_IDX, false, false);
    gpio_set_direction((gpio_num_t)m_TxPin, GPIO_MODE_OUTPUT);
    gpio_set_level((gpio_num_t)m_TxPin, 0); // Send BREAK

    // delayMicroseconds unreliable above 16383 see Arduino man pages
    delayMicroseconds(brakeBegin);
    // m_Serial.flush();
    gpio_set_level((gpio_num_t)m_TxPin, 1);
    delayMicroseconds(brakeEnd);

    m_Serial.begin(serialSpd);
}

/**
 * @brief Calculate checksum on LIN data
 * @param data bytes of data
 * @param data_size size of data
 * @return the calculated checksum
 */
int Lin::calculateChecksum(const unsigned char data[], uint8_t data_size) {
    int sum = 0;

    for (int i = 0; i < data_size; i++) {
        sum += data[i];
    }

    const uint8_t v_checksum = static_cast<uint8_t>(~(sum % 0xffu));
    return v_checksum;
}

/**
 * @brief Validate checksum according to enhanced crc
 * @param data - data array
 * @param data_size - size of array
 * @return whether the checksum is valid or not
 */
bool Lin::validateChecksum(const unsigned char data[], uint8_t data_size) {
    uint8_t checksum = data[data_size - 1];
    uint8_t v_checksum = calculateChecksum(data, data_size - 1);

    return checksum == v_checksum;
}
