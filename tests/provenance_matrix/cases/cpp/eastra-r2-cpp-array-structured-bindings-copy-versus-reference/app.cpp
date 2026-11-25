#include <cstdlib>
void observe(const char*, const char*);
void labels() {
    const char* pair[] = {std::getenv("EDITION"), "preview"};
    auto [snapshot, saved] = pair;
    auto& [live, other] = pair;
    live = "changed";
    observe(snapshot, live);
}
