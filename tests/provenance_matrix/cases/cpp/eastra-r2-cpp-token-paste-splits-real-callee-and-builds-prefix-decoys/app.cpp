#include <cstdlib>
#define JOIN(a, b) a ## b
static const char* getenv_cache() { return "preview"; }
static const char* query_cache() { return "catalog"; }
void observe(const char*, const char*, const char*, const char*);
void labels() {
    const char* direct = std::getenv("EDITION");
    const char* expanded = std::JOIN(get, env)("EDITION");
    const char* apparent = JOIN(getenv, _cache)();
    const char* generic = JOIN(query, _cache)();
    observe(direct, expanded, apparent, generic);
}
