def case():
    t = src()
    d = {}
    d["a"] = t
    with Manager():
        spare = 0
    sink(d)

class Manager:
    def __enter__(self):
        return self

    def __exit__(self, exc, value, tb):
        return False
