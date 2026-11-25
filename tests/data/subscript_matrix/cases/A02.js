function caseFn() {
    const t = src();
    const d = {"u": "s0"};
    d["u"] = {n: "s0"};
    d["u"].n = t; // W
    consume2(d["u"].n); // R
}

function consume2(p) {
    sink(p);
}
