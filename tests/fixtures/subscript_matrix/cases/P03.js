function caseFn() {
    const t = src();
    const d = {"a": "s0", "b": "s0"};
    d["a"] = t; // W
    d["a"] = "clean";
    sink(d["a"]); // R
}
