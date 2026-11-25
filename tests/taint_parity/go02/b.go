package main

import "os/exec"

func run(c string) {
	cmd := exec.Command(c) // @SINK
	cmd.Run()
}
