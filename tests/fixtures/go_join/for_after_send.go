package p
func f(a, b bool, i int, v interface{}, ch chan int, xs []int) int {
x := seed() // def
for a { // loop
g() // arm
}
ch <- 1
return x // use
}
