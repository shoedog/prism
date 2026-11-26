function caseFn() {
    const t = src();
    const o = {"a": "s0"};
    ({v: o["a"]} = {v: t}); // W
    let x;
    x = o["a"]; // R
    sink(x);
}
