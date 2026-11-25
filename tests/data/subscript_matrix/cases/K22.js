function caseFn() {
    const t = src();
    const o = {"é": "s0"};
    o.é = t; // W
    o["é"] = "clean";
    sink(o.é); // R
}
