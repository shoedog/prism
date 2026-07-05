class A:
    def m(self, p):
        sink(p)

def f(obj):
    user = input()
    obj.m(user)
