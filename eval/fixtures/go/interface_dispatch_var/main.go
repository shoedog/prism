package main

type Runner interface {
	Go()
}

type Fast struct{}

func (f Fast) Go() {}

func use() { _ = Fast{} }

func run() {
	var r Runner
	r.Go()
}
