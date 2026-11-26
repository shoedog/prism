#include <sys/types.h>
#include <sys/socket.h>
#include <sys/uio.h>

int read_first(int fd) {
    unsigned char bytes[8] = {0};
    struct iovec buffer = {.iov_base = bytes, .iov_len = sizeof bytes};
    struct msghdr message = {.msg_iov = &buffer, .msg_iovlen = 1};
    if (recvmsg(fd, &message, 0) <= 0) return -1;
    return bytes[0];
}
