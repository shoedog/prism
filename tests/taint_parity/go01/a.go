package main

import (
	"os"
	"os/exec"
)

func run(c string) {
	cmd := exec.Command(c) // @SINK
	cmd.Run()
}

func main() {
	t := os.Getenv("X") // @SRC
	run(t)
}
