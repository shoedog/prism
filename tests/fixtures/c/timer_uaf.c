#include <stdlib.h>

struct timer_ctx {
    void (*callback)(void *);
    void *data;
    int active;
};

void cancel_timer(struct timer_ctx *timer) {
    timer->active = 0;
    free(timer->data);
    // BUG: timer struct not freed, callback pointer still valid
    // If timer fires between free(data) and next check, use-after-free
}

void timer_tick(struct timer_ctx *timer) {
    if (timer->active) {
        timer->callback(timer->data);  // Use-after-free if cancel_timer ran
    }
}
