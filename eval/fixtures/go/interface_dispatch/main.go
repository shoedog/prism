package main

type Runner interface {
	Go()
}

type Fast struct{}

func (f Fast) Go() {}

func run(r Runner) {
	r.Go()
}
