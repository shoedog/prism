import requests
def make() -> requests.Session:
    note: (yield 0)
    return requests.Session()
def f(u):
    session = make()
    return session.get(u)
