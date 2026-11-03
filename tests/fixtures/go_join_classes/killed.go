package p
func f(x int, a bool, i int) int { // def
if a {
x = 1
} else {
x = 2
}
i--
return x // use
}
