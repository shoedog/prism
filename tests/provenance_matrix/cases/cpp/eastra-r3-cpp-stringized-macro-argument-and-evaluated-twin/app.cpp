#include <cstdlib>
#define SPELL(expr) #expr
#define EVAL(expr) (expr)
void publish(const char*, const char*);
void show() {
    const char* spelling = SPELL(std::getenv("EDITION"));
    const char* label = EVAL(std::getenv("EDITION"));
    publish(spelling, label);
}
