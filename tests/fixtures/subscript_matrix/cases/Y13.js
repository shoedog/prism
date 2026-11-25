function caseFn() {
    const t = src();
    const k = "x";
    const o = {x: "s0"};
    o.x = t; // W
    sink(o[k]); // R
}
