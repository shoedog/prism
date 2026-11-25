import requests
def fetch(replacement, u):
    globals()["requests"] = replacement
    return requests.get(u)
