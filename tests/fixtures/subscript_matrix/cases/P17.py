def case():
    t = src()
    d = {"a": t} # W
    sink(d["a"]) # R
