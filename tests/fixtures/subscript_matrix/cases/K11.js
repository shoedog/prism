function caseFn() {
    const t = src();
    const p = "a";
    const o = {"a": "s0"};
    o[`${p}`] = t; // W
    sink(o["a"]); // R
}
