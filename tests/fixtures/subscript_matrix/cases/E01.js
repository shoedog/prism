function caseFn() {
    const t = src();
    const d = {"u": {n: "s0"}};
    d["u"] = {n: t}; // W
    f(d["u"].n); // R
}

function f(p) {
    sink(p);
}
