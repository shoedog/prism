import requests
def f(s: requests.Session | None):
    return s.get("u")
