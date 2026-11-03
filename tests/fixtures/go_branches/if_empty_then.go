package p
func f(x int, n int, a bool, b bool, v interface{}, ch chan int) int {
if a {
} else if b {
x = 2 // def
}
return x // use
}
