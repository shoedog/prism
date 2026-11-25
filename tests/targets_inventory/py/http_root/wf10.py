import requests
def fetch(u, replacement):
    result = requests.get(u)
    requests.get = replacement
    return result
