function caseFn() {
    const t = src();
    const o = {"x": "s0"};
    o.x = t; // W
    sink(o["x"]); // R
}
