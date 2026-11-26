#include <cstdlib>
const char* label() {
    try {
        const char* edition = std::getenv("EDITION");
        throw edition;
    } catch (const char* captured) {
        return captured;
    }
}
