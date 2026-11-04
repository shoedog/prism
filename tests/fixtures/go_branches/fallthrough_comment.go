package p
func f(x int, n int) int {
switch n {
case 0:
x = 1
fallthrough
// comment between cases
case x:
use(x)
}
return x
}
