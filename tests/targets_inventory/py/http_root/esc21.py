import requests
def fetch(u, replacement):
    from requests import get
    result = get(u)
    requests.get = replacement
    return result
