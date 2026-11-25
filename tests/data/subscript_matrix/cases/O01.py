def case():
    t = src()
    d = {"a": "s0", "b": "s0"}
    d["a"] = t # W
    mutate(d) # R1
    sink(d) # R

def mutate(d):
    d["a"] = "safe"
