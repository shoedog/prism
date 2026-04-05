// Modeled after CVE-2022-28388 (CAN usb_8dev double kfree_skb), CVE-2024-26748 (cdns3)
#include <stdlib.h>

int setup_channel(int ch_id) {
    char *rx_buf = kmalloc(4096, 0);
    if (!rx_buf) return -1;
    char *tx_buf = kmalloc(4096, 0);
    if (!tx_buf) goto cleanup;

    int ret = configure_dma(ch_id, rx_buf, tx_buf);
    if (ret < 0) {
        kfree(tx_buf);     // inline free
        goto cleanup;       // BUG: cleanup also frees tx_buf
    }
    return 0;

cleanup:
    kfree(tx_buf);          // Double free if we came from configure_dma failure
    kfree(rx_buf);
    return -1;
}
