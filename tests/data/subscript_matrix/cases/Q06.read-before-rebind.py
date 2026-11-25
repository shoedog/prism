def case():
    t = src()
    d = {}
    d["a"] = t

    d = take(d)
    sink(d)

def take(arg):
    return arg
