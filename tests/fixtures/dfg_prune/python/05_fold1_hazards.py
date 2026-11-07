def tf(c):
    x = 1
    try:
        x = f()
    finally:
        use(x)

def sup():
    x = 1
    with suppress(E):
        x = f()
    use(x)

def fe(c):
    x = 1
    for i in c:
        x = 2
        break
    else:
        x = 3
    use(x)

def we(c):
    x = 1
    while c:
        x = 2
        continue
    else:
        x = 3
    use(x)

def gen():
    x = 1
    x = 2
    yield x
    use(x)

def ml():
    x = 1
    x = (
        2)
    use(x)

def st():
    x = 1
    *x, y = f()
    use(x)

def an():
    x = 1
    x: int = 2
    use(x)

def ex():
    x = 1
    try:
        f()
    except E as x:
        pass
    use(x)
