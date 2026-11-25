package main

import (
	"os"
	"os/exec"
)

func f(x string) string {
	return x
}

func main() {
	t := os.Getenv("X") // @SRC
	y := f(t)
	cmd := exec.Command(y) // @SINK
	cmd.Run()
}
