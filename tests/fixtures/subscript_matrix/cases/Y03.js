function caseFn() {
    const t = src();
    const k = "a";
    const d = {"a": "s0", "b": "s0"};
    d["a"] = t; // W
    sink(d[k]); // R
}
