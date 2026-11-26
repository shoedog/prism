import requests
def make() -> requests.Session:
    def nested(value=(yield 1)):
        pass
    return requests.Session()
def f(u):
    session = make()
    return session.get(u)
