// Modeled after CVE-2021-20322 (kernel hostname), CWE-170 class
#include <string.h>

void copy_hostname(const char *input, size_t input_len) {
    char hostname[64];
    strncpy(hostname, input, sizeof(hostname));
    // BUG: if input >= 64 bytes, hostname is NOT null-terminated
    // strlen(hostname) reads past the buffer
    int len = strlen(hostname);
    log_hostname(hostname, len);
}
