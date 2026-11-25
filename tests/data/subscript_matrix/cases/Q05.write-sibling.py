def case():
    t = src()
    o = Box()
    o.items = [[None]]
    o.other = {}
    o.items[0][0] = t
    o.other = []
    sink(o)

class Box:
    pass
