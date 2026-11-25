def case():
    t = src()
    a = "s0"
    make()["a"] = t # W
    sink(a) # R

def make():
    return {"a": "s0"}
