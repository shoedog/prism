def f(a, b, i):
    x = 1
    x = 2
    x += 1
    x = x + 1
    use(x)
    a, b = b, a
    use(a)
    x = [0]
    x[i] = 1
    use(x)
    p.f = 1
    p.f = 2
    use(p.f)
    y = z = 3
    use(y)
    n, m = g()
    n, m = h()
    use(n)
