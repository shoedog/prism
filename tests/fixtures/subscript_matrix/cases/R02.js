function caseFn() {
    const t = src();
    const d = {"a": "s0", "b": "s0"};
    d["a"] = t; // W
    f(d["a"]); // R
}

function f(p) {
    sink(p);
}
