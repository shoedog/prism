function caseFn() {
    const t = src();
    const elements = ["s0"];
    const target = {
        get x() {
            return elements[0] ?? "s0";
        },
        set x(value) {
            elements[0] = value;
            elements.length = 0;
        },
    }
    const o = new Proxy(target, {});
    o["x"] = t; // W
    sink(o.x); // R
}
