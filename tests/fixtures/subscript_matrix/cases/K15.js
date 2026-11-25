function caseFn() {
    const t = src();
    const o = {};
    o.__proto__ = {value: "s0"};
    o["__proto__"] = t; // W
    sink(o.__proto__); // R
}
