function caseFn() {
    const t = src();
    const k = "q";
    const d = {"q": "s0", "b": "s0"};
    d[k] = t; // W
    d["b"] = "safe";
    sink(d["b"]); // R
}
