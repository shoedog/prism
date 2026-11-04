package p
func f(a, b bool, i int, v interface{}, ch chan int, xs []int) int {
x := seed() // def
switch v.(type) {
case int:
g() // arm
}
z := 1
return x // use
}
