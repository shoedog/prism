package fixture
type S struct { f int }
var x int
func f() {
if x := value(); x > 0 { // d
use(x) // u
}
}
