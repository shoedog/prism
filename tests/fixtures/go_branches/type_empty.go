package p
func f(x int, n int, a bool, b bool, v interface{}, ch chan int) int {
switch v.(type) {
case int:
default:
x = 2 // kill
}
return x // use
}
