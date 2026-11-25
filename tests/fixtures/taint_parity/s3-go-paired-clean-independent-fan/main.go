package main

import (
    "os"
    "path/filepath"
    "strings"
)

func entry() {
    t := acquire() // @SRC
    u := acquire()
    p := filepath.Clean(t)
    if !strings.HasPrefix(p, "/fixture/") {
        return
    }
    os.Rename(p, u) // @SINK
}
func acquire() string { return "sample" }
func main() {}
