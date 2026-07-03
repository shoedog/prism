package main

type Command struct {
	Run func()
}

func h1() {}
func h2() {}
func h3() {}
func h4() {}

func register_a() *Command { return &Command{Run: h1} }
func register_b() *Command { return &Command{Run: h2} }
func register_c() *Command { return &Command{Run: h3} }
func register_d() *Command { return &Command{Run: h4} }

func invoke(cmd *Command) {
	cmd.Run()
}
