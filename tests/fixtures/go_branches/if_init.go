package p
func f(x int, n int, a bool, b bool, v interface{}, ch chan int) int {
if t := n; t > 0 {
x = 1 // def
} else if b {
x = 2
}
return x // use
}
