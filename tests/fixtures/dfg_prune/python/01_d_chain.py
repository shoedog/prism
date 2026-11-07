def f(c):
    x = 1
    x = 2
    use(x)
    x = 3
    if c:
        x = 4
    use(x)
    for x in []:
        pass
    use(x)
    x = 5
    for x in []:
        pass
    use(x)
    with open() as x:
        pass
    x = 6
    use(x)

def g():
    x = 1
    with f() as x:
        pass
    use(x)
