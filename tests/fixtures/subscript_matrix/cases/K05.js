function caseFn() {
    const t = src();
    const o = {"a-b": "s0", "a b": "s0"};
    o["a-b"] = t; // W
    o["a b"] = "s";
    sink(o["a b"]); // R
}
