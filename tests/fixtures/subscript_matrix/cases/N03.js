function caseFn() {
    const t = src();
    const o = {x: "s0"};
    const u = o;
    o["x"] = t; // W
    u["x"] = "safe";
    sink(o["x"]); // R
}
