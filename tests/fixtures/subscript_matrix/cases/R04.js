function caseFn() {
    const t = src();
    run.call({});
    function run() {
        this.d = {"a": "s0"};
        this.d["a"] = t; // W
        sink(this.d["a"]); // R
    }
}
