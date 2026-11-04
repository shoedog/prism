package p
type S struct { a int }
func f(p *S, a bool, i int) int {
q := p
q.a = 1 // def
if a {
g()
} else {
g()
}
i--
return p.a // use
}
