def case():
    t = src()
    d = {"a": "", "b": ""}
    d["a"] = t
    clean = {}
    take(clean, d)

def take(one, two):
    sink(two)
