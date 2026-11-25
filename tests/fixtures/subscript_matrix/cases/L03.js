function caseFn() {
    const t = src();
    const d = {"u": {v: "s0", w: "s0"}};
    d["u"] = {v: t}; // W
    d["u"].w = "x";
    sink(d["u"]); // R
}
