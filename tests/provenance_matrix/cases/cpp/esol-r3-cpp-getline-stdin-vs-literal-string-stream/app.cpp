#include <iostream>
#include <sstream>
#include <string>

std::string label() {
    std::istringstream sample("preview");
    std::string literal;
    std::string submitted;
    std::getline(sample, literal);
    std::getline(std::cin, submitted);
    return submitted.empty() ? literal : submitted;
}
