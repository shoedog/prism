def case():
    t = src()
    o = C()
    o.x = t # W
    o = C()
    sink(o.x) # R

class C:
    x = "s0"
