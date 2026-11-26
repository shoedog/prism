def case():
    t = src()
    d = {"a": "", "b": ""}
    d["a"] = t
    take(d["a"], d)

def take(p, q):
    sink(q)
