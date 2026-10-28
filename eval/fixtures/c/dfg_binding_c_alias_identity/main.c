void uninitialized_alias(int x) {
    int y;
    y = x;
    sink(x);
}

void initialized_alias(int x) {
    int y = x;
    sink(x);
}

void conditional_alias(int x) {
    int y;
    if (ready())
        y = x;
    sink(x);
}

void frr_shape(int match) {
    int from;
    if (ready())
        from = match;
    sink(match);
}
