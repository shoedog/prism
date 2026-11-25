package main
func caseFn() {
    t := src()
    d := map[string]string{"a": "s0"}
    d["a"] = t // W
    x := d["a"] // R
    sink(x)
}
