import requests
@staticmethod
def make():
    return requests.Session()
def f():
    s = make()
    return s.get("u")
