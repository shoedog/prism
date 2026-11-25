function caseFn() {
    const t = src();
    const d = {"a": "s0", "b": "s0"};
    (d as any)["a"] = t; // W
    sink(d["a"]); // R
}
