package p
func f(a, b bool, i int, v interface{}, ch chan int, xs []int) int {
x := seed() // def
for a { // loop
g() // probe
i++
}
return x // use
}
