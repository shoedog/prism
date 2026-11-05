package fixture

type A struct { b, c string }
type S struct { f, g string; a A }

func scenario(p *S, q *S, x string, y string, r A, c bool) {
	p.a.b = x
	f := func() {
	p.a = r
	use(p)
	}
	f()
}
