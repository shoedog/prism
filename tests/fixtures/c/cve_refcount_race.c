// Modeled after CVE-2022-3640 (Bluetooth L2CAP refcount), CVE-2023-3609 (cls_u32)
#include <pthread.h>

struct shared_obj {
    int refcount;
    pthread_mutex_t lock;
    void *data;
};

void obj_get(struct shared_obj *obj) {
    // BUG: refcount modified without lock — TOCTOU race
    obj->refcount++;
}

void obj_put(struct shared_obj *obj) {
    pthread_mutex_lock(&obj->lock);
    obj->refcount--;
    if (obj->refcount == 0) {
        free(obj->data);
        free(obj);
    }
    pthread_mutex_unlock(&obj->lock);
}
