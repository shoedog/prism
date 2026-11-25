function caseFn() {
    const t = src();
    const o = {"x": "s0"};
    o["x"] = t; // W
    let u;
    u = o; // R1
    u["x"] = "safe";
    sink(o); // R
}
