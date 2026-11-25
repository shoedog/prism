#include <cstdlib>

const char* load() {
    const auto mode = std::getenv("MODE");
    using ReturnType = decltype(mode);
    return static_cast<ReturnType>(mode);
}
