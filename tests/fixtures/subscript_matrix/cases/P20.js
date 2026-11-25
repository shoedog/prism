function caseFn() {
    const t = src();
    const xs = [t]; // W
    sink(xs[0]); // R
}
