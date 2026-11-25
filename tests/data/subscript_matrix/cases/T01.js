function caseFn() {
    const t = src();
    const o = {"a": "s0"};
    for (o["a"] of [t]) {} // W
    let x;
    x = o["a"]; // R
    sink(x);
}
