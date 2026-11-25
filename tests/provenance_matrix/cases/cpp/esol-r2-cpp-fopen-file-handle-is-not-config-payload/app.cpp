#include <cstdio>
std::FILE* open_catalog() {
    std::FILE* handle = std::fopen("catalog.txt", "r");
    return handle;
}
