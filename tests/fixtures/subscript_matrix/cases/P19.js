function caseFn() {
    const t = src();
    const d = {"a": t, "b": "safe"}; // W
    sink(d["b"]); // R
}
