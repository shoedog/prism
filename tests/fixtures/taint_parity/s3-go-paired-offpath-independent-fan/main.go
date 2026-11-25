package main

import (
    "os"
    "path/filepath"
    "strings"
)

func entry() {
    t := acquire() // @SRC
    u := acquire()
    p := filepath.Clean(u)
    if !strings.HasPrefix(p, "/fixture/") { return }
    observe(p)
    os.Open(t) // @SINK
}
func observe(string) {}
func acquire() string { return "sample" }
func main() {}
