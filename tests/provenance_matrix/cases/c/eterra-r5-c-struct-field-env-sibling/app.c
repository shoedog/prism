#include <stdlib.h>
struct Pair { const char *left; const char *right; };
void run(void) {
  struct Pair pair = { getenv("MODE"), "fixed" };
  const char *value = pair.right;
  (void)value;
}
