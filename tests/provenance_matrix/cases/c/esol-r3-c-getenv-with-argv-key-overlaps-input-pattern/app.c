#include <stdlib.h>

const char *label(int argc, char **argv) {
    const char *selected = getenv(argc > 1 ? argv[1] : "PREVIEW_LABEL");
    return selected;
}
