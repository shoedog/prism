package p
func f(x int, a bool, b bool) int {
if a {
x = 1
} else if x > 0 && b { // condition use
x = 2
} else {
x = 3
}
return x
}
