function caseFn() {
    const t = src();
    const d = {"a": t}; // W
    sink(d["a"]); // R
}
