function caseFn() {
    const t = src();
    sink(h(t));
}

function h(t) {
    const d = {"u": {n: "s0"}};
    d["u"] = {n: t}; // W
    return d["u"].n; // R
}
