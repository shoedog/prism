package p
func f(x int, n int, a bool, b bool, v interface{}, ch chan int) int {
select {
default:
x = 1 // def
}
return x // use
}
