def case():
    t = src()
    d = Hashable(); table = {d: "clean"}
    d["a"] = t
    x = table[d]
    sink(x)

class Hashable(dict):
    __hash__ = object.__hash__
