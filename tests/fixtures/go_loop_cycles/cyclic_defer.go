package p

func f(x int, c bool) int {
    for c {
        defer done()
        use(x) // use
        x = 1 // def
    }
    return x
}
