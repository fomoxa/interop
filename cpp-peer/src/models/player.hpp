#pragma once

#include <cstdint>

#include "fomoxa.h"

namespace models {

FOMOXA_MODEL
FOMOXA_CODEC("edge")
struct Player
{
    FOMOXA_FIELD(u32)
    FOMOXA_CODEC("edge")
    uint32_t Id = 0;

    FOMOXA_FIELD(f32)
    FOMOXA_CODEC("edge")
    float X = 0.0f;

    FOMOXA_FIELD(f32)
    FOMOXA_CODEC("edge")
    float Y = 0.0f;
};

}  // namespace models
