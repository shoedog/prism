function caseFn() {
    const t = src();
    const d = {"a": "s0"};
    const c = true;
    if (c) {
        d["a"] = t; // W
    } else {
        d["a"] = "s";
    }
    sink(d["a"]); // R
}
