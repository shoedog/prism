import requests
def fetch(u, replacement):
    globals()["requests"].get = replacement
    return requests.get(u)
