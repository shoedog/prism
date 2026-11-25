#include <stdlib.h>
const char *edition(void) {
    const char *label = getenv("EDITION");
#if 0
    label = "sample";
#endif
    return label;
}
