#include <stdlib.h>
#include <string.h>
#include <stdint.h>

typedef struct {
    uint8_t *payload;
    size_t len;
} frame_t;

int validate_header(frame_t *frame);
void dispatch_frame(frame_t *frame);

void process_frame(uint8_t *raw, size_t len) {
    frame_t *frame = malloc(sizeof(frame_t));
    frame->payload = malloc(len);
    memcpy(frame->payload, raw, len);

    if (validate_header(frame) < 0) {
        free(frame->payload);
        free(frame);
        goto cleanup;
    }

    dispatch_frame(frame);
    return;

cleanup:
    free(frame->payload);  // BUG: double free if validation failed
    free(frame);
}
