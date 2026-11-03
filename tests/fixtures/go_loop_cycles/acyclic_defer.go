package p

func f(x int) int {
    defer done()
    goto Assign
Read:
    return x // use
Assign:
    x = 1 // def
    goto Read
}
