import requests
requests.get = lambda *_: 7

def read():
    value = requests.get("https://example.invalid")
    return value
