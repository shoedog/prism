package main

type Runner interface {
	Go()
}

type Fast struct{}

func (f Fast) Go() {}

func use() { _ = Fast{} }

func run(x any) {
	x.(Runner).Go()
}
