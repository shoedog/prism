function caseFn() {
    const t = src();
    const k = "x";
    const d = {"x": {"a": "s0"}};
    d[k]["a"] = t; // W
    sink(d["x"]["a"]); // R
}
