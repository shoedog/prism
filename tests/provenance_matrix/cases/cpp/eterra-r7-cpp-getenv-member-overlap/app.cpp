#include <cstdlib>
struct Config { const char* mode; };
const char* load(Config cfg) {
  const char* mode = std::getenv("MODE");
  return cfg.mode;
}
