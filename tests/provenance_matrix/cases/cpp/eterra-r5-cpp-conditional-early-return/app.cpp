#include <cstdlib>
const char* run(bool flag) {
  const char* mode = std::getenv("MODE");
  if (flag) { mode = "fixed"; return mode; }
  return mode;
}
