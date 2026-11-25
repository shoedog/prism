#include <fcntl.h>
#include <linux/rtc.h>
#include <sys/ioctl.h>
#include <unistd.h>
void record(int, int);
void sample(void) {
    int fd = open("/dev/rtc0", O_RDONLY);
    if (fd < 0) return;
    struct rtc_time sample;
    int rc = ioctl(fd, RTC_RD_TIME, &sample);
    close(fd);
    if (rc < 0) return;
    int hour = sample.tm_hour;
    record(hour, rc);
}
