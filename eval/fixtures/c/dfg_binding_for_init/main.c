void f() {
    int x = source();
    for (int x = clean(); ready(); step()) {
        sink(x);
    }
    sink(x);
}
