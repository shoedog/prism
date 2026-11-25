#include <cstdlib>
const char* edition = std::getenv("EDITION");
const char* choose(const char* value =
    edition) { return value; }
void publish(const char*, const char*);
void show() {
    const char* edition = "preview";
    publish(choose(), edition);
}
