def case():
    t = src()
    o = Box()
    o.items = [[None]]
    o.other = {}
    o.items[0][0] = t
    x = o.items[0]
    sink(x)

class Box:
    pass
