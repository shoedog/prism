def tf():
    x = 1
    try:
        x = 2
    finally:
        use(x)

def sup():
    x = 1
    with suppress(E):
        x = f()
    use(x)

def forelse(c):
    x = 1
    for i in c:
        x = 2
        break
    else:
        pass
    use(x)

def gen():
    x = 1
    x = 2
    yield
    use(x)

def ml():
    x = 1
    x = (
        2)
    use(x)

def star(t):
    x = 1
    a, *x = t
    use(x)

def ann():
    x = 1
    x: int = 2
    use(x)

def exc():
    x = 1
    try:
        pass
    except E as x:
        pass
    use(x)

def whl(c):
    x = 1
    while c():
        x = 2
        continue
    else:
        pass
    use(x)
