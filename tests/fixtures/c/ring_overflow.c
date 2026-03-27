#include <stdint.h>
#include <string.h>

#define RING_SIZE 256
static uint8_t ring_buf[RING_SIZE];
static volatile int write_idx = 0;

void ring_write(uint8_t *data, int count) {
    // BUG: No wrap-around check, writes past buffer end
    memcpy(ring_buf + write_idx, data, count);
    write_idx += count;
    // Should be: write_idx = (write_idx + count) % RING_SIZE
}
