def f(q):
    p = q
    p.x = 1
    sink(q.x)
    p = q
