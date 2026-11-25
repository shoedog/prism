function caseFn() {
    const t = src();
    const d = {"a": "s0", "b": "s0"};
    d["a"] = t; // W
    mutate(d);
    sink(d["a"]); // R
}

function mutate(d) {
    d["a"] = "safe";
}
