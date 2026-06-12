package main

func target() {}

func run() {
	f := func() { target() }
	f()
}
