def h(p):
    sink(p)

def g(p):
    h(p)

def f(p):
    g(p)

def top():
    user = input()
    f(user)
