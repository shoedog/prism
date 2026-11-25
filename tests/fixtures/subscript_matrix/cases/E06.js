function caseFn() {
    const t = src();
    const d = {"u": {"n": "s0", "m": "s0"}};
    d["u"] = {"n": t}; // W
    sink(d["u"]["n"]); // R
}
