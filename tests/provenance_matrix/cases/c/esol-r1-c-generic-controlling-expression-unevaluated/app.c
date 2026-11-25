#include <stdlib.h>

const char *load(void) {
    const char *label = _Generic(getenv("LABEL"), char *: "preview", default: "fixed");
    return label;
}
