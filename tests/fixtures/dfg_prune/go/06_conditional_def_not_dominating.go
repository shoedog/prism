package fixture
type S struct { f int }
var x int
func f(c bool) {
x = 1 // a
if c {
x = 2 // b
}
use(x) // u
}
