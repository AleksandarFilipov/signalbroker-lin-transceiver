#pragma once

#include <cstdint>

namespace
{
    constexpr bool LOG_TO_SERIAL = false;
    constexpr uint8_t UDP_TX_PACKET_MAX_SIZE_CUSTOM = 128u;

    template <typename Input>
    uint8_t value(Input t)
    {
        return static_cast<uint8_t>(t);
    }
} // namespace