#include <sys/socket.h>
long read_count(int fd) {
    char buffer[16];
    long count = recv(fd, buffer, sizeof(buffer), 0);
    return count;
}
