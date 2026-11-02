package fixture
type S struct { f int }
var x int
func f() {
x = 1 // a
{
var x int // b
use(x) // u
x = 3 // c
}
}
