#include <cstdlib>
#include <utility>
void observe(const char *, const char *);
void update() {
    const char *live = std::getenv("EDITION");
    const char *old = std::exchange(live, "sample");
    observe(old, live);
}
