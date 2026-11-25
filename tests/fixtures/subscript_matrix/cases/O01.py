def case():
    t = src()
    d = {"a": "s0", "b": "s0"}
    d["a"] = t # W
    mutate(d)
    sink(d["a"]) # R

def mutate(d):
    d["a"] = "safe"
