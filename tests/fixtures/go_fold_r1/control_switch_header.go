package p
func f(n int) int {
switch v := n + 1; n { // header
case 0:
println(v) // first
case 1:
println(v) // second
}
return n
}
