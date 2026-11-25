#include <linux/io.h>

unsigned int read_state(const volatile void *register_address) {
    unsigned int state = readl(register_address);
    return state;
}
