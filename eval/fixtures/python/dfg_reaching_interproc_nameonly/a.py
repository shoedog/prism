class C:
    def callee(self, p):
        sink(p)

def caller(obj):
    x = source()
    obj.callee(x)
