package p
func f(x int, n int, a bool, b bool, v interface{}, ch chan int) int {
if a {
x = 1
} else if b {
x = 2 // def
} else {
x = 3 // sibling kill
}
return x // use
}
