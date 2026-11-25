#include <stdlib.h>
void report(const char *);

char *read_value(void) {
    char *p = 0;
    report(p);
    p = getenv("MODE");
    return p;
}
