function caseFn() {
    const t = src();
    const k = "a";
    const d = {"a": "s0", "b": "s0"};
    d[k] = t; // W
    sink(d["a"]); // R
}
