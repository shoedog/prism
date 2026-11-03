package p
func f(x int, a bool, n int) int {
for a {
x = 1 // def
continue
}
switch n {
case 0:
default:
x = 2 // sibling kill
}
return x // use
}
