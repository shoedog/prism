function caseFn() {
    const t = src();
    const d = make();
    d["a"] = t; // W
    sink(make()["a"]); // R
}

function make() {
    return {a: "s0"};
}
