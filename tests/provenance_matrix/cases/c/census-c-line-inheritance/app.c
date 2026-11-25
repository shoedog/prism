#include <stdlib.h>
void report(const char *, const char *);

void read_value(void) {
    const char *label = "mode";
    report(label, getenv("MODE"));
}
