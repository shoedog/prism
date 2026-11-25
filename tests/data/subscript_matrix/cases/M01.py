def case():
    t = src()
    o = {"x": C(), "y": C()}
    i = "x"
    j = "y"
    o[j].z = "safe"
    o[i].z = t # W
    sink(o[j].z) # R

class C:
    z = "s0"
