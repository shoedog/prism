#include <sys/socket.h>
void report(char *);

long read_count(int fd) {
    char buffer[16] = "fixed";
    report(buffer);
    long count = recv(fd, buffer, sizeof(buffer), 0);
    report(buffer);
    return count;
}
