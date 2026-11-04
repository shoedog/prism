package p
func f(a, b bool, i int, v interface{}, ch chan int, xs []int) int {
x := seed() // def
for range xs { // loop
{
use(i) // target
}
g() // probe
}
return x // use
}
