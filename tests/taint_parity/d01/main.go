package main

import "os/exec"

type Runner interface {
	Run(c string)
}

type A struct{}
type B struct{}

func (a A) Run(c string) {
	cmd := exec.Command(c) // @SINK
	_ = cmd
}

func (b B) Run(c string) {
	println(c)
}

func use(r Runner) {
	t := getInput() // @SRC
	r.Run(t)
}

func getInput() string { return "" }

func main() { use(A{}); use(B{}) }
