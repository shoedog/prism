import requests
def make() -> requests.Session:
    return requests.Session()
def f():
    s = make()
    return s.get("u")
def g():
    global make
    make = None
