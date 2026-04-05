// Modeled after CVE-2026-23162 (xe driver double-free/leak in goto path)
#include <stdlib.h>
#include <stdint.h>

struct ploam_ctx { uint8_t *buf; uint8_t *key; };

int handle_activate(struct ploam_ctx *ctx, uint8_t *msg) {
    ctx->buf = kmalloc(256, 0);
    if (!ctx->buf) return -1;
    ctx->key = kmalloc(32, 0);
    if (!ctx->key) goto err_buf;
    int ret = decrypt_ploam(ctx->key, msg);
    if (ret < 0) goto err_buf;  // BUG: should be err_key — leaks ctx->key
    return 0;
err_key:
    kfree(ctx->key);
err_buf:
    kfree(ctx->buf);
    return -1;
}
