#include <cstdlib>

const char *caption() {
    const char *label = "preview";
    if constexpr (false) {
        label = std::getenv("PREVIEW_LABEL");
    }
    return label;
}
