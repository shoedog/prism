package fixture

type A struct { b, c string }
type S struct { f, g string; a A }

func scenario(p *S, x string) *S {
	p.f = x
	return p
}
