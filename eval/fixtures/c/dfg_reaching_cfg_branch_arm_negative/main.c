void f(int c) {
    int x = source();
    if (c) {
        x = clean();
    } else {
        sink(x);
    }
}
