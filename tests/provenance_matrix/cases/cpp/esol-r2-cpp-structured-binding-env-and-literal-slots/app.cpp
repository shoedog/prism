#include <cstdlib>
#include <utility>
void consume(const char*, const char*);
void load() {
    auto [mode, caption] = std::pair{std::getenv("MODE"), "Catalog"};
    consume(mode, caption);
}
