package p
func f(a, b bool, i int, v interface{}, ch chan int, xs []int) int {
x := seed() // def
for range xs { // loop
g() // probe
{
use(i) // target
}
}
return x // use
}
