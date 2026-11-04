package p
func f(n int, cond bool) int {
var source int // outer
switch n {
case 0:
source = 7
case 1:
println(n)
}
if cond {
println(n)
} else {
var typed int // declaration
println(n)
typed = source // use
println(typed)
}
return source
}
