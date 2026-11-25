import requests
if True:
    def make() -> requests.Session:
        return requests.Session()
def f(u):
    session = make()
    return session.get(u)
