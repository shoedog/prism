function caseFn() {
    const t = src();
    const d = {"a": "s0", "b": "s0"};
    d["b"] = "s"; // W
    f(d["b"]); // R
}

function f(p) {
    sink(p);
}
