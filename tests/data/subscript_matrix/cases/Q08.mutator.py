def case():
    t = src()
    d = {"a": "", "b": ""}
    d["a"] = t
    mutate(d)
    sink(d)

def mutate(value):
    value["a"] = "clean"
