#include <cstdlib>

const char* load(const char* mode) {
    if (const char* mode = std::getenv("MODE"); mode != nullptr) {
        return mode;
    }
    return mode;
}
