function caseFn() {
    const t = src();
    const o = {"a": "s0"};
    [o["a"]] = [t]; // W
    let x;
    x = o["a"]; // R
    sink(x);
}
