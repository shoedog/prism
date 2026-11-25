function caseFn() {
    const t = src();
    const o = {items: {a: "s0"}};
    o.items["a"] = t; // W
    sink(o.items); // R
}
