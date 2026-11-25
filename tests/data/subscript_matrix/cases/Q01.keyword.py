def case():
    t = src()
    d = {"a": "", "b": ""}
    d["a"] = t
    take(value=d)

def take(value):
    sink(value)
