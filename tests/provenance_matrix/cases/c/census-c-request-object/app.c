struct incoming { int status; };

struct incoming *expose(struct incoming *request) {
    return request;
}
