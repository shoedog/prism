def case():
    t = src()
    d = {"a": "", "b": ""}
    d["a"] = t
    clean = {}
    take(d, clean)

def take(one, two):
    sink(two)
