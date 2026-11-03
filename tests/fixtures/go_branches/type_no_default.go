package p
func f(x int, n int, a bool, b bool, v interface{}, ch chan int) int {
switch v.(type) {
case int:
x = 1 // def
}
return x // use
}
