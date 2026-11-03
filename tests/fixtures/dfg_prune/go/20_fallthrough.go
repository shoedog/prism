package fixture
func keep(x int, n int) {
switch n {
case 1:
x = 1
fallthrough
case 2:
use(x)
}
}
func overwrite(x int, n int) {
switch n {
case 1:
x = 1
fallthrough
case 2:
x = 2
use(x)
}
}
