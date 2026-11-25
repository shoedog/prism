def case():
    t = src()
    d = {"a": "", "b": ""}
    make()["a"] = t
    sink(d)

def make():
    return {}
