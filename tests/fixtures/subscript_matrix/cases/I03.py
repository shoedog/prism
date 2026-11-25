def case():
    t = src()
    base = {"a": t} # W
    d = {**base, "b": "s"}
    sink(d["a"]) # R
