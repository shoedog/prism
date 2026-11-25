function caseFn() {
    const t = src();
    const d = {
        "a": ( // W
            t
        ),
        "b": "s",
    };
    sink(d["a"]); // R
}
