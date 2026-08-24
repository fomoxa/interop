#pragma once

#include <stdint.h>

#include "fomoxa.h"

FOMOXA_MODEL
FOMOXA_CODEC("edge")
struct Player {
    FOMOXA_FIELD(u32)
    FOMOXA_CODEC("edge")
    uint32_t Id;

    FOMOXA_FIELD(f32)
    FOMOXA_CODEC("edge")
    float X;

    FOMOXA_FIELD(f32)
    FOMOXA_CODEC("edge")
    float Y;
};
