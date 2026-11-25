def case():
    t = src()
    d = {}
    def inner():
        d["a"] = t
    inner()
    sink(d)
