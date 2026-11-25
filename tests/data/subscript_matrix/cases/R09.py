def case():
    t = src()
    make()["a"] = t # W
    clean = {"a": "s0"}
    x = clean # R
    sink(x)

def make():
    return {"a": "s0"}
