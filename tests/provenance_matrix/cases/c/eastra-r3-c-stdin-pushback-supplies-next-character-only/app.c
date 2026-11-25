#include <stdio.h>
void record(int, int);
void labels(void) {
    if (ungetc('P', stdin) == EOF) return;
    int prefix = getchar();
    int payload = getchar();
    if (payload == EOF) return;
    record(prefix, payload);
}
