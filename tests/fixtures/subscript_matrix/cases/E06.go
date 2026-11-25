package main
func caseFn() {
    t := src()
    d := map[string]map[string]string{"u": {"n": "s0", "m": "s0"}}
    d["u"] = map[string]string{"n": t} // W
    sink(d["u"]["n"]) // R
}
