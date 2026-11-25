function caseFn() {
    const t = src();
    const o = {"x": "s0"};
    o.x = t; // W
    o["x"] = "clean";
    sink(o.x); // R
}
