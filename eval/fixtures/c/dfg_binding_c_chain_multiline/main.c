void continuation_line() {
    int a = 1,
        x = 2;
    sink(x);
}

void later_assignment() {
    int a, x;
    x = source();
    sink(x);
}
