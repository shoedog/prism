void pointer_definition() {
    int *p = source();
    sink(p);
}

void uninitialized_pointer() {
    char *buf;
    buf = source();
    sink(buf);
}

void function_pointer_definition() {
    int (*fp)(int) = source();
    sink(fp);
}

void const_pointer_definition() {
    const char *s = source();
    sink(s);
}

void pointer_to_pointer_definition() {
    int **pp = source();
    sink(pp);
}

void array_definition() {
    int a[4];
    sink(a);
}
