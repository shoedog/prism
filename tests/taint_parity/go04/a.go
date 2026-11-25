package main

import (
	"os"
	"os/exec"
)

func run(c string) {
	exec.Command(c).Run() // @SINK
}

func main() {
	t := os.Getenv("X") // @SRC
	run(t)
}
