#include <stdint.h>

typedef struct {
    uint8_t type;
    uint8_t subtype;
    uint8_t flags;
    uint16_t len;
    uint8_t data[128];
} msg_t;

int dispatch_message(msg_t *msg) {
    switch (msg->type) {
        case 0x01:
            if (msg->flags & 0x01) {
                if (msg->subtype == 0xA0) return 10;
                else if (msg->subtype == 0xA1) return 11;
                else return -1;
            } else {
                if (msg->subtype == 0xA0) return 20;
                else return -1;
            }
        case 0x02:
            if (msg->len > 64) return -1;
            switch (msg->subtype) {
                case 0xB0: return 30;
                case 0xB1: return 31;
                case 0xB2: return 32;
                case 0xB3: return 33;
                case 0xB4: return 34;
                case 0xB5: return 35;
                case 0xB6: return 36;
                case 0xB7: return 37;
                default: return -1;
            }
        case 0x03:
            switch (msg->flags) {
                case 0x00: return 40;
                case 0x01: return 41;
                case 0x02: return 42;
                case 0x03:
                    if (msg->subtype == 0xC0) return 43;
                    else if (msg->subtype == 0xC1) return 44;
                    else return -1;
                default: return -1;
            }
        case 0x04:
            if (msg->subtype == 0xD0) {
                switch (msg->flags) {
                    case 0: return 50;
                    case 1: return 51;
                    case 2: return 52;
                    default: return -1;
                }
            } else if (msg->subtype == 0xD1) {
                return 60;
            } else {
                return -1;
            }
        case 0x05:
            return msg->data[0];
        case 0x06:
            return msg->data[0] + msg->data[1];
        case 0x07:
            return msg->data[0] * msg->data[1];
        case 0x08:
            if (msg->len == 0) return -1;
            return msg->data[msg->len - 1];
        case 0x09:
            return 90;
        case 0x0A:
            return 100;
        default:
            return -1;
    }
}
