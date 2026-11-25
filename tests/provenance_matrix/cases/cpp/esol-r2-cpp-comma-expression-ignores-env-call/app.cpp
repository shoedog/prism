#include <cstdlib>
const char* load() {
    const char* caption = (static_cast<void>(std::getenv("MODE")), "Catalog");
    return caption;
}
