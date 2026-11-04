package fixture
type S struct { f string }
func scenario(p *S, x string) {
 p.f = x
 useValue(*p)
}
