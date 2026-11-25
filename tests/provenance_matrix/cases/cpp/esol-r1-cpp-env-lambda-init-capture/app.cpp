#include <cstdlib>

const char* load() {
    auto lookup = [mode = std::getenv("MODE")] {
        return mode;
    };
    return lookup();
}
