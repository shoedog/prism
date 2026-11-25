def case():
    t = src()
    d = {"a": "", "b": ""}
    d["a"] = t
    take(d, d["a"])

def take(p, q):
    sink(q)
