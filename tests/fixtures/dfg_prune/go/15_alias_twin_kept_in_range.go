package fixture
type S struct { f int }
var x int
func f(s *S) {
ptr := s
s.f = 1 // a
ptr.f = 2 // b
use(s.f) // u
}
