function caseFn() {
    const t = src();
    const o = {"1": "s0"};
    o[1] = t; // W
    sink(o["1"]); // R
}
