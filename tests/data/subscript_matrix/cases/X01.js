function caseFn() {
    const t = src();
    const o = {"k": "s0"};
    o["k"] = t; // W
    delete o["k"];
    sink(o); // R
}
