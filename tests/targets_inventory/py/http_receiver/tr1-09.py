import requests
def f():
    s = requests.Session()
    s += 1
    return s.get("u")
