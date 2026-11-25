import requests
def make() -> requests.Session:
    class Marker((yield 1)):
        pass
    return requests.Session()
def f(u):
    session = make()
    return session.get(u)
