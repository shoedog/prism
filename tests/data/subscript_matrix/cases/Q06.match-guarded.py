def case():
    t = src()
    d = {}
    d["a"] = t
    match [1]:
        case [item] if item > 0:
            spare = item
    sink(d)
