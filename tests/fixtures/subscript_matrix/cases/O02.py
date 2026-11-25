def case():
    t = src()
    d = R()
    d["a"] = t # W
    sink(d["b"]) # R

class R(dict):
    def __init__(self):
        self.slot = "s0"

    def __setitem__(self, key, value):
        self.slot = value

    def __getitem__(self, key):
        return self.slot
