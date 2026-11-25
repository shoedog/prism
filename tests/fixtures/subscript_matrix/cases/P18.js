function caseFn() {
    const t = src();
    const d = {
        "a": t, // W
        "b": "safe"};
    sink(d["b"]); // R
}
