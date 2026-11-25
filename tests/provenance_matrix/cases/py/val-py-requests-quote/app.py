import requests

def read():
    value = requests.utils.quote("red blue")
    return value
