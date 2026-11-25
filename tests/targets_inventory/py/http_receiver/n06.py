import requests


def fetch(u):
    s = requests.Session()
    result = s.get(u)
    s = other()
    return result
