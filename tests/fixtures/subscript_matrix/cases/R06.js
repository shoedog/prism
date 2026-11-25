function caseFn() {
    const t = src();
    const d = {"a": "s0"};
    let x = "s0";
    for (let i = 0; i < 2; i++) {
        x = d["a"]; // R
        d["a"] = t; // W
    }
    sink(x);
}
