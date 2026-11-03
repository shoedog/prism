package p
func f(a, b bool, i int, v interface{}, ch chan int, xs []int) int {
x := seed() // def
if a {
g() // probe
{
use(i) // target
}
} else if b {
g() // arm
} else {
g() // arm
}
return x // use
}
