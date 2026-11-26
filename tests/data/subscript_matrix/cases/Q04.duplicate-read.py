def case():
    t = src()
    d = {"a": "", "b": ""}
    d["a"] = t
    take(d, d)

def take(p, q):
    sink(q)
