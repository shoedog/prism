function caseFn() {
    const t = src();
    const o = {"a": "s0"};
    ({v: o["a"]} = {v: t}); // W
    sink(o["a"]); // R
}
