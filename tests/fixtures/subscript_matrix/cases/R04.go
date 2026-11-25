package main
func caseFn() {
    t := src()
    self := S{d: map[string]string{"a": "s0"}}
    self.d["a"] = t // W
    sink(self.d["a"]) // R
}

type S struct {
    d map[string]string
}
