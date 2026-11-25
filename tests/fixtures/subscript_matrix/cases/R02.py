def case():
    t = src()
    d = {"a": "s0", "b": "s0"}
    d["a"] = t # W
    f(d["a"]) # R

def f(p):
    sink(p)
