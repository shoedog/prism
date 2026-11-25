#include <cstdlib>
const char* load() {
    const char* mode = std::getenv /* obtain deployment mode */ ("MODE");
    return mode;
}
