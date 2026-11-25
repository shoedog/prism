def case():
    t = src()
    self = C()
    self.d = {"a": "s0"}
    self.d["a"] = t # W
    sink(self.d["a"]) # R

class C:
    pass
