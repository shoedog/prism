#include <stdint.h>
#include <string.h>

#define DECLARE_HANDLER(name) \
    int handle_##name(uint8_t *buf, size_t len)

#define HANDLE_ERROR(ret, msg) \
    do { if ((ret) < 0) { return -1; } } while(0)

#define MIN(a, b) ((a) < (b) ? (a) : (b))
#define MAX(a, b) ((a) > (b) ? (a) : (b))
#define CLAMP(v, lo, hi) MIN(MAX((v), (lo)), (hi))

#ifdef DEBUG
#define LOG(fmt, ...) debug_print(fmt, ##__VA_ARGS__)
#else
#define LOG(fmt, ...) do {} while(0)
#endif

#define VALIDATE_LEN(len, min_len) \
    do { if ((len) < (min_len)) return -1; } while(0)

typedef int (*handler_fn)(uint8_t *, size_t);

DECLARE_HANDLER(data);
DECLARE_HANDLER(control);
DECLARE_HANDLER(status);

static handler_fn dispatch_table[256];

int handle_data(uint8_t *buf, size_t len) {
    VALIDATE_LEN(len, 4);
    uint8_t type = buf[0];
    size_t payload_len = CLAMP(buf[1], 0, len - 2);
    LOG("handle_data: type=%d len=%zu", type, payload_len);
    HANDLE_ERROR(process_data_payload(buf + 2, payload_len), "payload error");
    return 0;
}

int handle_control(uint8_t *buf, size_t len) {
    VALIDATE_LEN(len, 2);
    uint8_t cmd = buf[0];
#ifdef EXTENDED_CMDS
    if (cmd >= 0x80) {
        return handle_extended_cmd(buf, len);
    }
#endif
    return dispatch_table[cmd] ? dispatch_table[cmd](buf + 1, len - 1) : -1;
}

int handle_status(uint8_t *buf, size_t len) {
    VALIDATE_LEN(len, 1);
    return (int)buf[0];
}
