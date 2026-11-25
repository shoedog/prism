function caseFn() {
    const t = src();
    const k = "q";
    const o = {"q": "s0"};
    o[k] = t; // W
    sink(o.q); // R
}
