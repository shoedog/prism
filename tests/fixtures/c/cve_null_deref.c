// Modeled after CVE-2024-26600 (phy-omap-usb2 NULL ptr), CVE-2023-28328 (az6027)
#include <stdint.h>

struct buffer { void *data; int len; };

void init_buffers(int count) {
    struct buffer *bufs = kmalloc(count * sizeof(struct buffer), 0);
    // BUG: no NULL check — kmalloc can return NULL under memory pressure
    bufs[0].data = kmalloc(1024, 0);
    bufs[0].len = 1024;
}
