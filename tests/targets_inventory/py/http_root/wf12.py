import requests
def fetch(u, replacement, setattr):
    setattr(requests, "get", replacement)
    return requests.get(u)
