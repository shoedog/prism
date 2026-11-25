import requests
def fetch(requests: requests.Session, /, u):
    return requests.get(u)
