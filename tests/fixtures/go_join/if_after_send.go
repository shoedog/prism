package p
func f(a, b bool, i int, v interface{}, ch chan int, xs []int) int {
x := seed() // def
if a {
g() // arm
}
ch <- 1
return x // use
}
