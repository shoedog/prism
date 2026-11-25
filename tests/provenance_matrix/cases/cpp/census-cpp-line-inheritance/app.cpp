#include <cstdio>
void report(int, const char *);

void read_value(void) {
    int label = 7;
    char buffer[16];
    report(label, fgets(buffer, sizeof(buffer), stdin));
}
