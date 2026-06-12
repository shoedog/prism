package main

type Engine struct{}

func (e Engine) Start() {}

func run(e Engine) {
	e.Start()
}
