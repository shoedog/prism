#include <fcntl.h>
#include <sys/ioctl.h>
int open_peer(int master) {
    int peer = ioctl(master, TIOCGPTPEER, O_RDONLY);
    return peer;
}
