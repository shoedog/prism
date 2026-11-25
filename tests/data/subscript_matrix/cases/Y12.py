def case():
    t = src()
    k = "b"
    d = {"a": "s0", "b": "s0"}
    d["a"] = t # W
    f(d["b"], d[k]) # R

def f(p, q):
    sink(q)
