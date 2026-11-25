#include <cstdlib>
#include <iostream>
void run() {
  const char *mode = std::getenv("MODE");
  mode = "fixed";
  std::cout << mode;
}
