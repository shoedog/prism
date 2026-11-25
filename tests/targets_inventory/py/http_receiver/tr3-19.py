import requests
def f():
    s = requests.Session()
    def inner():
        return s.get("u")
    return inner
