package p

func f(x int, c bool) int {
Read:
    use(x) // use
    x = 1 // def
    if c {
        goto Read
    }
    return x
}
