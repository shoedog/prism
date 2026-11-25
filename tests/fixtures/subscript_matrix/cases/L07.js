function caseFn() {
    const t = src();
    const o = {"a-b": "s0", "x": "s0"};
    o["a-b"] = t; // W
    o.x = "c";
    sink(o["a-b"]); // R
}
