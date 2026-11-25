from .helpers import make
def f(u):
    session = make()
    return session.get(u)
