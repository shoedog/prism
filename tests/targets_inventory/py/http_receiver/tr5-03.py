import requests
def f():
    s: requests.Session = requests.Session()
    return s.get("u")
