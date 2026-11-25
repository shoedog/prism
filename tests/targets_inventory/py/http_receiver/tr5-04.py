import requests
def make() -> requests.Session:
    def nested():
        yield 1
    return requests.Session()
def f():
    s = make()
    return s.get("u")
