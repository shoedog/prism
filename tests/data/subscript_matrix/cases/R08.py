def case():
    t = src()
    d = {"a": "s0"}
    c = True
    if c:
        d["a"] = t # W
    else:
        d["a"] = "s"
    sink(d) # R
