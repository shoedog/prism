#include <cstdlib>
void consume(const char*);
void run(bool enabled) {
  const char* mode = "default";
  if (enabled) mode = std::getenv("MODE");
  consume(mode);
}
