package p
func f(x int, n int, a bool, b bool, v interface{}, ch chan int) int {
if a {
x = 1 // def
}
return x // use
}
