#include <iostream>
#include <sstream>
#include <string>
#include <utility>
std::pair<std::string, std::string> labels() {
    std::string live, preview;
    if (!std::getline(std::cin, live)) return {};
    std::istringstream fixed("preview\n");
    auto* previous = std::cin.rdbuf(fixed.rdbuf());
    std::getline(std::cin, preview);
    std::cin.rdbuf(previous);
    return {live, preview};
}
