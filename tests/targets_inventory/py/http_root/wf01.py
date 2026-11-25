import requests
def fetch(u):
    del requests.get
    return requests.get(u)
