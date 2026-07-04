package main

type Runner interface {
	Go()
}

type Impl struct{}

func (i Impl) Go() {}

var r Runner

func run() {
	r.Go()
}
