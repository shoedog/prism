// Modeled after CVE-2023-3812 (tun oversize packet), CVE-2017-14491 (dnsmasq)
#include <stdlib.h>
#include <string.h>

struct msg_buf { char *data; size_t len; size_t cap; };

void append_data(struct msg_buf *buf, const char *chunk, size_t chunk_len) {
    if (buf->len + chunk_len > buf->cap) {
        // BUG: chunk_len from network — realloc with attacker-controlled size
        buf->data = realloc(buf->data, buf->len + chunk_len);
        buf->cap = buf->len + chunk_len;
    }
    memcpy(buf->data + buf->len, chunk, chunk_len);
    buf->len += chunk_len;
}
