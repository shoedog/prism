#include <map>

bool contains(const std::map<int, int>& entries) {
    const auto match = entries.find(3);
    return match != entries.end();
}
