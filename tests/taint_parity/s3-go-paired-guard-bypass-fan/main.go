package main

import (
    "os"
    "path/filepath"
    "strings"
)

func entry(alternate bool, bypass bool) {
    t := acquire() // @SRC
    u := acquire()
    path := filepath.Clean(t)
    if !strings.HasPrefix(path, "/fixture/") { return }
    if bypass { path = t }
    if alternate { path = u }
    os.Open(path) // @SINK
}
func acquire() string { return "sample" }
func main() {}
