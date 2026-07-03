class A:
    @property
    def text(self):
        return 1


class B:
    @property
    def text(self):
        return 2


class C:
    @property
    def text(self):
        return 3


class D:
    @property
    def text(self):
        return 4


def f(r):
    return r.text
