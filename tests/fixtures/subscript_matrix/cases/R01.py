def case():
    t = src()
    d = make()
    d["a"] = t # W
    sink(make()["a"]) # R

def make():
    return {"a": "s0"}
