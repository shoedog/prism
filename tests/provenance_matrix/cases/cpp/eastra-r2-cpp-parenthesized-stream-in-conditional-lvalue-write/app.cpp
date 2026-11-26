#include <iostream>
#include <string>
void observe(const std::string&, const std::string&);
void labels(bool first) {
    std::string left = "left", right = "right";
    if (!(std::cin >> (first ? left : right))) return;
    if (first) {
        observe(left, right);
    } else {
        observe(right, left);
    }
}
