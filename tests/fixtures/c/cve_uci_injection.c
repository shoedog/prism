// Modeled after CVE-2024-54143 (OpenWrt ASU command injection)
#include <stdlib.h>

void set_hostname(const char *user_input) {
    char cmd[512];
    snprintf(cmd, sizeof(cmd), "uci set system.@system[0].hostname='%s'", user_input);
    system(cmd);  // BUG: user_input not sanitized — shell injection via ' escaping
    system("uci commit system");
}
