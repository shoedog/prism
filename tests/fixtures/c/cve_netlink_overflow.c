// Modeled after CVE-2023-3390 (nf_tables), CVE-2022-1972 (netfilter)
#include <string.h>
#include <stdint.h>

struct nlmsghdr { uint32_t nlmsg_len; uint16_t nlmsg_type; };

void handle_netlink_msg(struct nlmsghdr *nlh) {
    char buf[4096];
    size_t payload_len = nlh->nlmsg_len - sizeof(struct nlmsghdr);
    // BUG: nlmsg_len < sizeof(nlmsghdr) wraps to huge value
    memcpy(buf, (char *)nlh + sizeof(struct nlmsghdr), payload_len);
}
