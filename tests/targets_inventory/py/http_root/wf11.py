import requests
def fetch(u, replacement, change):
    if change:
        requests.get = replacement
    return requests.get(u)
