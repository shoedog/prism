package main
func caseFn() {
    t := src()
    o := S{items: map[string]string{"a": "s0"}}
    o.items["a"] = t // W
    sink(o.items) // R
}

type S struct {
    items map[string]string
}
