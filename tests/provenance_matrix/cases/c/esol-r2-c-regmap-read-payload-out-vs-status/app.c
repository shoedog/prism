#include <linux/regmap.h>
void consume(unsigned int state, int rc);
void load(struct regmap *map) {
    unsigned int state = 0;
    int rc = regmap_read(map, 4, &state);
    if (rc) return;
    consume(state, rc);
}
