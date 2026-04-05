// Modeled after CVE-2020-25682, CVE-2020-25683 (dnsmasq memcpy negative size)
#include <string.h>
#include <stdint.h>

void parse_tlv(uint8_t *pdu, size_t pdu_len) {
    int length = (int16_t)(pdu[2] << 8 | pdu[3]);  // Signed from network
    if (length > 1024) return;  // BUG: negative length passes this check
    char buf[1024];
    memcpy(buf, pdu + 4, length);  // Implicit cast to size_t — huge positive
}
