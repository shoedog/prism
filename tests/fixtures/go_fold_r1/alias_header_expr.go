package p
func f(x int, value interface{}) int {
switch saved := x; saved {
case 0:
x = 3 // write
}
return x // result
}
