package p
func f(value interface{}, n int) {
for n > 0 {
switch v := value.(type) { // alias
case int:
println(v) // integer
case string:
println(v) // string
}
}
}
