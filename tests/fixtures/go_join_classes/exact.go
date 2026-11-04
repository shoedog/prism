package p
func f(a bool, i int) int {
x := 1 // def
if a {
g()
} else {
g()
}
i--
return x // use
}
