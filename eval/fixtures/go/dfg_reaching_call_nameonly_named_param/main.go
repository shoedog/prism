package main
type Command struct { Run func(int) }
func worker(p int) { sink(p) }
func New() Command { return Command{Run: worker} }
func invoke() {
	c := New()
	x := source()
	c.Run(x)
}
