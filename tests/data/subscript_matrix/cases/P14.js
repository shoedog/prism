function caseFn() {
    const t = src();
    const d = {"a": "s0", "b": "s0"};
    d["a"] = t; // W
    sink(d); // R
}
