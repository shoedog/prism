package p
func f(x int, n int, a bool) int {
for a {
x = 2 // kill
switch n {
case 0:
x = 1 // def
break
case x: // use
return x
}
}
return x
}
