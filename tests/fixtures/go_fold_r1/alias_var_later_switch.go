package p
func f(n, limit int) int {
i := 0 // outer
for i < limit { // header
var saved int
saved = i // saved
switch n {
case 0:
i = saved + 1 // write
default:
i += 2 // second
}
}
return i // result
}
