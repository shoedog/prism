def case():
    t = src()
    o = C(t) # W
    sink(o.a) # R

class C:
    def __init__(self, v):
        self.a = v
