function caseFn() {
    const t = src();
    const d = {"u": {n: "s0"}, "w": {n: "s0"}};
    d["u"] = {n: t}; // W
    sink(d["w"].n); // R
}
