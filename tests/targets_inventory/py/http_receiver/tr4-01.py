import requests
def make():
    return requests.Session()
def f():
    s = make()
    s = None
    return s.get("u")
