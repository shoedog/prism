function caseFn() {
    const t = src();
    const o = {"x": "s0", "y": "s0"};
    o.x = t; // W
    o["y"] = "c";
    sink(o.x); // R
}
