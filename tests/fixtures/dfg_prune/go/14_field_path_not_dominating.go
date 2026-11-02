package fixture
type S struct { f int }
var x int
func f(p S, q S) {
p.f = 1 // a
p = q // b
use(p.f) // u
}
