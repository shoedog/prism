void block_scope() {
    int x = source();
    {
        int a = 1, x = 2;
        sink(x);
    }
    sink(x);
}

void for_scope() {
    int x = source();
    for (int a = 1, x = 2; ready(); step()) {
        sink(x);
    }
    sink(x);
}
