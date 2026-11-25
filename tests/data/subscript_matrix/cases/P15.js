function caseFn() {
    const t = src();
    const d = {"a": "s0"};
    d["a"] = t; // W
    const x = d["a"]; // R
    sink(x);
}
