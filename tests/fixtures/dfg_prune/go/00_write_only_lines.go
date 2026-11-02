package fixture
type S struct { f int }
var x int
func f(x int, p S) {
x = 1 // a
x = 2 // b
x += 1 // aug
x++ // inc
y := x // read
var z int // zero
z = x // zw
p.f = 1 // fa
p.f = 2 // fb
use(p.f) // fr
use(y, z)
var v S // vdef
v.f = 1 // vw1
v.f = 2 // vw2
use(v)
}
