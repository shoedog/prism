typedef const char *Text;

struct S {
    Text x;
};

void case_fn(void) {
    Text t = src();
    struct S buf[1] = {{"s0"}};
    int i = 0;
    buf[i].x = t; // W
    sink(buf[i].x); // R
}
