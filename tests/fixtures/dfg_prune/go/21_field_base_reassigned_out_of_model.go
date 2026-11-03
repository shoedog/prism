package fixture
type T struct { f int }
// Pointer identity is out of model (design section 2.2): paths are fixed storage.
func f(a, b *T) int {
p := a
p.f = 1
p = b
p.f = 2
p = a
return p.f
}
