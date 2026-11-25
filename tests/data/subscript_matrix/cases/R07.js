function caseFn() {
    const t = src();
    const d = {a: "s0"};
    d["a"] = t; // W
    function g() {
        sink(d); // R
    }
    g();
}
