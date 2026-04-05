// Modeled after CVE-2023-0179 (nft_payload VLAN), CVE-2020-14386 (net/packet)
#include <stdint.h>
#include <string.h>

void parse_options(uint8_t *pdu, size_t pdu_len) {
    uint8_t option_count = pdu[4];
    char options[32][64];
    // BUG: <= allows option_count == 32, which writes options[32] (out of bounds)
    for (int i = 0; i <= option_count; i++) {
        memcpy(options[i], pdu + 5 + i * 64, 64);
    }
}
