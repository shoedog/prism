#include <iostream>
void observe(int, std::streamsize);
void preview() {
    int next = std::cin.peek();
    auto count = std::cin.gcount();
    if (next == std::char_traits<char>::eof()) return;
    observe(next, count);
}
