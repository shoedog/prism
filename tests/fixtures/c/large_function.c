#include <stdint.h>
#include <string.h>

#define MAX_CHANNELS 16
#define BUF_SIZE 512

typedef struct {
    uint8_t id;
    uint8_t state;
    uint32_t counter;
    uint8_t buf[BUF_SIZE];
} channel_t;

static channel_t channels[MAX_CHANNELS];

int process_channel_data(int ch_id, uint8_t *data, size_t len, int flags) {
    if (ch_id < 0 || ch_id >= MAX_CHANNELS) return -1;
    channel_t *ch = &channels[ch_id];

    if (flags & 0x01) {
        if (ch->state == 0) {
            ch->state = 1;
            ch->counter = 0;
            memset(ch->buf, 0, BUF_SIZE);
        } else if (ch->state == 1) {
            ch->counter++;
            if (ch->counter > 100) {
                ch->state = 2;
            }
        } else {
            ch->state = 0;
        }
    }

    if (flags & 0x02) {
        if (len > BUF_SIZE) len = BUF_SIZE;
        memcpy(ch->buf, data, len);
    }

    if (flags & 0x04) {
        for (int i = 0; i < (int)len; i++) {
            if (data[i] == 0xFF) {
                ch->state = 0;
                break;
            } else if (data[i] == 0xFE) {
                ch->counter = 0;
            } else if (data[i] == 0xFD) {
                if (ch->state > 0) ch->state--;
            } else if (data[i] == 0xFC) {
                ch->counter++;
            } else if (data[i] == 0xFB) {
                memset(ch->buf, data[i], BUF_SIZE);
            } else if (data[i] == 0xFA) {
                for (int j = 0; j < BUF_SIZE; j++) {
                    ch->buf[j] ^= data[i];
                }
            } else if (data[i] == 0xF9) {
                ch->state = 3;
            } else if (data[i] == 0xF8) {
                ch->state = 4;
            } else if (data[i] == 0xF7) {
                ch->state = 5;
            } else if (data[i] == 0xF6) {
                ch->state = 6;
            } else if (data[i] == 0xF5) {
                ch->state = 7;
            } else if (data[i] == 0xF4) {
                if (i + 1 < (int)len) {
                    ch->buf[0] = data[i + 1];
                }
            } else if (data[i] == 0xF3) {
                if (i + 2 < (int)len) {
                    ch->buf[0] = data[i + 1];
                    ch->buf[1] = data[i + 2];
                }
            } else if (data[i] == 0xF2) {
                if (ch->counter < 0xFFFFFFFF) ch->counter++;
            } else if (data[i] == 0xF1) {
                if (ch->counter > 0) ch->counter--;
            } else if (data[i] == 0xF0) {
                ch->id = (uint8_t)(ch->counter & 0xFF);
            }
        }
    }

    if (flags & 0x08) {
        switch (ch->state) {
            case 0: ch->counter = 0; break;
            case 1: ch->counter += 10; break;
            case 2: ch->counter += 20; break;
            case 3: ch->counter += 30; break;
            case 4: ch->counter += 40; break;
            case 5: ch->counter += 50; break;
            case 6: ch->counter += 60; break;
            case 7: ch->counter += 70; break;
            default: ch->counter = 0xDEAD; break;
        }
    }

    if (flags & 0x10) {
        if (ch->state == 0 && ch->counter == 0) {
            return 0;
        } else if (ch->state == 1) {
            if (ch->counter < 50) {
                return 1;
            } else if (ch->counter < 100) {
                return 2;
            } else {
                return 3;
            }
        } else if (ch->state == 2) {
            return 4;
        } else if (ch->state >= 3) {
            return (int)ch->state;
        }
    }

    if (flags & 0x20) {
        for (int i = 0; i < BUF_SIZE - 1; i++) {
            if (ch->buf[i] > ch->buf[i + 1]) {
                uint8_t tmp = ch->buf[i];
                ch->buf[i] = ch->buf[i + 1];
                ch->buf[i + 1] = tmp;
            }
        }
    }

    if (flags & 0x40) {
        uint32_t sum = 0;
        for (int i = 0; i < BUF_SIZE; i++) {
            sum += ch->buf[i];
        }
        ch->counter = sum;
    }

    if (flags & 0x80) {
        if (len >= 4) {
            uint32_t magic = (data[0] << 24) | (data[1] << 16) | (data[2] << 8) | data[3];
            if (magic == 0xDEADBEEF) {
                memset(ch->buf, 0, BUF_SIZE);
                ch->state = 0;
                ch->counter = 0;
            } else if (magic == 0xCAFEBABE) {
                ch->state = 1;
                ch->counter = 42;
            } else if (magic == 0xFEEDFACE) {
                ch->state = 2;
                ch->counter = 99;
            }
        }
    }

    return (int)(ch->state + ch->counter % 256);
}
