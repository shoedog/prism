from requests import Session
def make() -> Session:
    return Session()
def f(u):
    return make().get(u)
