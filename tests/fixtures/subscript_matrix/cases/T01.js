function caseFn() {
    const t = src();
    const o = {"a": "s0"};
    for (o["a"] of [t]) {} // W
    sink(o["a"]); // R
}
