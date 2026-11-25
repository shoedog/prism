function caseFn() {
    const t = src();
    const k = "b";
    const d = {"a": "s0", "b": "s0"};
    d["a"] = t; // W
    f(d["b"], d[k]); // R
}

function f(p, q) {
    sink(q);
}
