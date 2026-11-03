package fixture
func exitDefer(x int, c bool) {
 defer use(x)
 for c {
  x = 1
  break
 }
}
func nextDefer(x int, c bool) {
 for c {
  x = 1
  break
 }
 defer use(x)
}
