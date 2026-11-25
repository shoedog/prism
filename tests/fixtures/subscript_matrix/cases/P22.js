function caseFn() {
    const t = src();
    class C {
        n = "s0";
        m = "s0";
        c = "s0";
    }
    const d = {"u": "s0"};
    d["u"] = new C();
    d["u"].n = t; // W
    d["u"].m = "safe";
    sink(d["u"].m); // R
}
