// Modeled after CVE-2019-5188 (e2fsprogs), CVE-2005-0101 (syslog format string injection)
#include <syslog.h>
#include <string.h>

void handle_login(const char *username) {
    char msg[256];
    snprintf(msg, sizeof(msg), "Login attempt: %s", username);
    syslog(LOG_INFO, msg);  // BUG: msg used as format string, not syslog(LOG_INFO, "%s", msg)
}
