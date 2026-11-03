package p
type S struct { a int }
func f(a bool, p *S) int {
x := 1 // def
select {}
return x // use
}
