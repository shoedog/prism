#include <cstdlib>
#include <map>
#include <string>
#include <utility>
std::pair<const char*, const char*> labels() {
    std::map<std::string, const char*> entries{{"current", "preview"}};
    entries.try_emplace("current", std::getenv("EDITION"));
    const char* first = entries.at("current");
    entries.insert_or_assign("current", std::getenv("EDITION"));
    const char* second = entries.at("current");
    return {first, second};
}
