import requests
def f():
    s = requests.Session()
    del s
    return s.get("u")
