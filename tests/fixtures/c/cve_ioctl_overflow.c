// Modeled after CVE-2022-20409 (io_uring), kernel ioctl class
#include <stdint.h>

struct user_request { uint32_t size; uint8_t data[]; };

int handle_ioctl(unsigned long arg) {
    struct user_request req;
    copy_from_user(&req, (void *)arg, sizeof(req));
    char *buf = kmalloc(req.size, 0);  // req.size from userspace, unchecked
    // BUG: second copy uses user-controlled size with no upper bound
    copy_from_user(buf, (void *)arg + sizeof(req), req.size);
    return process(buf, req.size);
}
