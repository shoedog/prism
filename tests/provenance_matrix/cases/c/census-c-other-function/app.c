#include <stdlib.h>
const char *setting(void) {
    const char *mode = getenv("MODE");
    return mode;
}
const char *label(void) {
    const char *mode = "ready";
    return mode;
}
