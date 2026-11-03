package p
func f(a, b bool, i int, v interface{}, ch chan int, xs []int) int {
x := seed() // def
for range xs { // loop
i++
g() // probe
}
return x // use
}
