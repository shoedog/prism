function caseFn() {
    const t = src();
    const k = "z";
    const o = {z: "s0", x: "s0"};
    o[k] = t; // W
    o.x = "c";
    sink(o[k]); // R
}
