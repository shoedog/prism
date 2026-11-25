#include <cstdlib>
void f() {
  const char* mode = std::getenv("MODE");
  auto show = [mode]() { return mode; };
  (void)show;
}
