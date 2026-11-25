package main
func caseFn() {
    t := src()
    d := map[string]string{
        "a": t, // W
        "b": "s",
    }
    sink(d["a"]) // R
}
