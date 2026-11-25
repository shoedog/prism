#include <cstdlib>
void use(const char*);
void load() {
  for (const char* mode = std::getenv("MODE"); mode; ) {
    use(mode);
    break;
  }
}
