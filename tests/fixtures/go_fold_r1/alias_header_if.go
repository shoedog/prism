package p
func f(x int, value interface{}) int {
if saved := x; saved > 0 {
x = 3 // write
}
return x // result
}
