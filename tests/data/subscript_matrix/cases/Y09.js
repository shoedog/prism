function caseFn() {
    const t = src();
    const xs = ["s0", "s0"];
    xs[0] = t; // W
    sink(xs); // R
}
