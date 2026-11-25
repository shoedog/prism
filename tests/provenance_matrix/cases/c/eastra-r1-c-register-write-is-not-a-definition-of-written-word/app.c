#include <linux/io.h>
void update(volatile void *address, unsigned int command, unsigned int *out) {
    unsigned int previous = readl(address);
    writel(command, address);
    out[0] = previous;
    out[1] = command;
}
