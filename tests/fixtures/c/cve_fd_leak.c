// Modeled after CWE-775; ubiquitous in OpenWrt daemons (uhttpd, netifd)
#include <stdio.h>
#include <stdlib.h>

int read_config(const char *path, char *buf, size_t bufsize) {
    FILE *f = fopen(path, "r");
    if (!f) return -1;
    if (fgets(buf, bufsize, f) == NULL) {
        fclose(f);
        return -1;
    }
    // BUG: fclose(f) missing on success path
    return 0;
}
