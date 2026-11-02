package fixture
type S struct { f int }
var x int
func f() {
x = 1 // a
go func() {
use(x) // u
}()
x = 2 // b
}
