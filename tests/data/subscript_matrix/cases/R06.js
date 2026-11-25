function caseFn() {
    const t = src();
    const d = {a: "s0"};
    let x = {};
    for (let i = 0; i < 2; i++) {
        if (i === 1) {
            x = d; // R
            break;
        }
        d["a"] = t; // W
    }
    sink(x);
}
