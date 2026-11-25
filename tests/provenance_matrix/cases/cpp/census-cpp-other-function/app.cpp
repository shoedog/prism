#include <cstdlib>
struct Sources {
    const char *setting() {
        const char *mode = getenv("MODE");
        return mode;
    }
    const char *label() {
        const char *mode = "ready";
        return mode;
    }
};
