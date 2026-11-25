#include <unistd.h>
void consume(int, const char*);
void load(int argc, char** options) {
    int status = getopt(argc, options, "m:");
    if (status == 'm') {
        const char* mode = optarg;
        consume(status, mode);
    }
}
