def case():
    t = src()
    d = {
        "a": t, # W
        "b": "safe"}
    sink(d["b"]) # R
