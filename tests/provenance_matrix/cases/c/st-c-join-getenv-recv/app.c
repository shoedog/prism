#include <stdlib.h>
#include <sys/socket.h>

char *read_value(int fd, int remote) {
    static char buffer[16];
    char *value;
    if (remote) {
        recv(fd, buffer, sizeof(buffer), 0);
        value = buffer;
    } else {
        value = getenv("MODE");
    }
    return value;
}
