#include <stdio.h>
void sample(void) {
    int edition, consumed;
    int assigned = scanf("%*d%d%n", &edition, &consumed);
    if (assigned != 1) return;
    printf("%d %d %d\n", edition, consumed, assigned);
}
