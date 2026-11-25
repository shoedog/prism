function caseFn() {
    const t = src();
    const d = {"u": {n: "s0"}};
    d["u"] = {n: t}; // W
    const x = d["u"].n; // R
    sink(x);
}
