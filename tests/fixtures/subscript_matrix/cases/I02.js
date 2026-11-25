function caseFn() {
    const t = src();
    const d = {
        "a": t, // W
        "b": "s",
    };
    sink(d["b"]); // R
}
