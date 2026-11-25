function caseFn() {
    const t = src();
    const d = {"u": new Map([["n", "s0"]])};
    d["u"] = new Map([["n", t]]); // W
    sink(d["u"].get("n")); // R
}
