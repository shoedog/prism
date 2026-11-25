import requests
def make() -> requests.Session:
    return requests.Session()
make = None
def f():
    s = make()
    return s.get("u")
