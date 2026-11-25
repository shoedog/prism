package main

import (
    "os"
    "path/filepath"
)

func entry() {
    t := acquire() // @SRC
    path := t
    if !filepath.IsLocal(path) { path = "fixture.txt" }
    os.Open(path) // @SINK
}
func acquire() string { return "sample" }
func main() {}
