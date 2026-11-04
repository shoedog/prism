package p
func f(x int, value interface{}) int {
switch saved := x; value.(type) {
case int:
_ = saved
x = 3 // write
}
return x // result
}
