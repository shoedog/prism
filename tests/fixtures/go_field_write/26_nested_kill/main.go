package fixture

type A struct { b, c string }
type S struct { f, g string; a A }

func scenario(p *S, x string) {
	p.a.b = x
	p.a = A{}
	useA(p.a)
	use(p)
}
