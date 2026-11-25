import requests
def fetch(replacement, u):
    requests.get = replacement
    return requests.get(u)
