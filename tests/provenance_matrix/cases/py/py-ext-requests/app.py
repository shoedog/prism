import requests

def load(url):
    resp = requests.get(url)
    payload = resp.json()
    return payload
