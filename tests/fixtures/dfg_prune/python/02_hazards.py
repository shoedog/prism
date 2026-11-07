x = 0

def g():
    global x
    x = 1
    x = 2
    use(x)

def h():
    x = 0
    def w():
        nonlocal x
        x = 9
    x = 1
    w()
    use(x)

def t():
    x = 1
    try:
        x = f()
    except E:
        use(x)

def wal():
    x = 1
    if (x := f()):
        pass
    use(x)

def dl():
    x = 1
    del x
    x = 2
    use(x)

def nested():
    x = 1
    x = 2
    k = lambda: x
    l = [x for _ in y]
    return k, l

class C:
    x = 1
    def m(self):
        use(x)
        x = 2
        use(x)
