def case():
    t = src()
    d = Discard()
    d["a"] = t
    sink(d)

class Discard(dict):
    def __setitem__(self, key, value):
        pass
