function caseFn() {
    const t = src();
    const d = {"u": "s0"};
    d["u"] = {n: "s0"};
    d["u"].n = t; // W
    consume({slot: d["u"].n}); // R
}

function consume(p) {
    sink(p.slot);
}
