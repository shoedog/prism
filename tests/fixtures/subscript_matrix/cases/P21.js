function caseFn() {
    const t = src();
    const d = {"a": "s0"};
    d["a"] = t; // W
    d["a"] += "x";
    sink(d["a"]); // R
}
