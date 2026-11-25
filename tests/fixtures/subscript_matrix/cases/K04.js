function caseFn() {
    const t = src();
    const o = {"a-b": "s0"};
    o["a-b"] = t; // W
    sink(o["a-b"]); // R
}
