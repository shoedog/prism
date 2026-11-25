function caseFn() {
    const t = src();
    const i = "x";
    const j = "x";
    const d = {"x": {"a": "s0"}, "y": {"a": "s0"}};
    d[i]["a"] = t; // W
    sink(d[j]["a"]); // R
}
