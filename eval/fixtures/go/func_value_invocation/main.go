package main

type Command struct {
	Run func()
}

func helper() {}

func register() *Command {
	return &Command{Run: helper}
}

func invoke(cmd *Command) {
	cmd.Run()
}
