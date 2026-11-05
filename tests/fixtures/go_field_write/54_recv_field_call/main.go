package fixture

type A struct { b, c string }; type R struct { Run func(); f string }
type S struct { f, g string; a A }

func scenario(p *R, x string) {
	p.f = x
	p.Run()
}
