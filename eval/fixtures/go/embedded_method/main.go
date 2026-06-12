package main

type Base struct{}

func (b Base) Ping() {}

type Wrap struct {
	Base
}

func run(w Wrap) {
	w.Ping()
}
