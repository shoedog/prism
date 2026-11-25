import requests
def fetch(u):
    del globals()["requests"]
    return requests.get(u)
