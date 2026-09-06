def callee(p):
    sink(p)

def caller():
    x = source()
    callee(x)
