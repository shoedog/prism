function caseFn() {
    const t = src();
    const o = {"x": "s0"};
    o["\x78"] = t; // W
    sink(o.x); // R
}
