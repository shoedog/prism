package fixture
type S struct { f int }
var x int
func f() {
a, err := source() // a
b, err := source() // b
use(err) // u
use(a, b)
}
