package main

type Command struct {
	Run func()
}

func helper() {}

func main() {
	c := Command{Run: helper}
	_ = c
}
