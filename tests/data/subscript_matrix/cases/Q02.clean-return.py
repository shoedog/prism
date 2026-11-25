def case():
    t = src()
    d = {"a": "", "b": ""}
    d["a"] = t
    v = take(d)
    sink(v)

def take(value):
    return {}
