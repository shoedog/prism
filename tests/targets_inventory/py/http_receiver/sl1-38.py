import requests
def make() -> requests.Session | None:
    return requests.Session()
def f(u):
    session = make()
    return session.get(u)
