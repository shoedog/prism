// Modeled after CVE-2023-42753 (ipset missing bound check), CVE-2023-35001 (nft)
#include <stdint.h>

typedef void (*handler_fn)(uint8_t *data);
handler_fn handlers[8];

void dispatch_message(uint8_t *pdu) {
    uint8_t msg_type = pdu[0];
    // BUG: no bounds check — msg_type can be 0-255, table has 8 entries
    handlers[msg_type](pdu + 1);
}
