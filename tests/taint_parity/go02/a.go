package main

import "os"

func main() {
	t := os.Getenv("X") // @SRC
	run(t)
}
