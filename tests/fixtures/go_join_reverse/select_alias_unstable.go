package p
type S struct { a int }
func f(a bool, p *S) int {
q := p
q.a = 1 // def
select {}
return p.a // use
}
