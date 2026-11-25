function caseFn() {
    const t = src();
    const o = {"a": "s0"};
    [o["a"]] = [t]; // W
    sink(o["a"]); // R
}
