function caseFn() {
    const t = src();
    put({a: t, b: "safe"}); // W
}

function put(q) {
    sink(q["b"]); // R
}
