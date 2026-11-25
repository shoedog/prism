def case():
    t = src()
    d = {"a": "", "b": ""}
    d[(d := {}) or "a"] = t
    sink(d)
