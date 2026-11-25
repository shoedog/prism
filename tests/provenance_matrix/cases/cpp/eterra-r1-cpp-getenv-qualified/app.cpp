#include <cstdlib>
#include <iostream>
void run() {
  const char *mode = std::getenv("MODE");
  std::cout << mode;
}
