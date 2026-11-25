function caseFn() {
    const t = src();
    const o = {"é": "s0"};
    o["é"] = t; // W
    sink(o.é); // R
}
