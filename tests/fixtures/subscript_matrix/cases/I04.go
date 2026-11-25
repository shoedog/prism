package main
func caseFn() {
    t := src()
    d := map[string]string{
        "a": ( // W
            t),
        "b": "s",
    }
    sink(d["a"]) // R
}
