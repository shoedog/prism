package fixture
type S struct { f int }
var x int
func f() {
use(x) // u
x = 2 // d
}
