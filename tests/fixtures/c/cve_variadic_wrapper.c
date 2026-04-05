// Modeled after CVE-2019-11477/11478 (TCP SACK kernel logging)
#include <stdio.h>
#include <stdarg.h>

void device_log(int level, const char *fmt, ...) {
    va_list args;
    va_start(args, fmt);
    char buf[512];
    vsnprintf(buf, sizeof(buf), fmt, args);
    va_end(args);
    write_log(level, buf);
}

void handle_request(const char *user_data) {
    device_log(3, user_data);  // BUG: user_data as format string
}
