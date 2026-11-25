#include <cstdlib>
#include <tuple>
void observe(const char *, const char *);
int main(int argc, char **argv) {
    if (argc < 2) return 0;
    const char *first, *last;
    std::tie(first, std::ignore, last) =
        std::make_tuple("sample", argv[1], std::getenv("EDITION"));
    observe(first, last);
}
